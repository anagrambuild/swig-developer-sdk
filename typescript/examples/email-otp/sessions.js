import { jwtVerify } from 'jose';
import { randomBytes } from 'node:crypto';

export class LoginSessions {
  #sessions = new Map();

  async issue(idToken, attempt, state, issuer, clientId, jwks) {
    if (
      !attempt ||
      attempt.expires <= Date.now() ||
      state !== attempt.state ||
      typeof idToken !== 'string'
    )
      throw new Error('Invalid login attempt');
    const { payload } = await jwtVerify(idToken, jwks, {
      issuer,
      audience: clientId,
      algorithms: ['EdDSA'],
      requiredClaims: ['sub', 'exp', 'iat', 'nonce'],
    });
    if (
      payload.nonce !== attempt.oidcNonce ||
      typeof payload.sub !== 'string' ||
      !payload.sub
    )
      throw new Error('Invalid login claims');
    const id = randomBytes(32).toString('hex');
    const expires = Math.min(payload.exp * 1000, Date.now() + 5 * 60_000);
    this.#sessions.set(id, { subject: payload.sub, expires });
    return {
      id,
      maxAge: Math.max(0, Math.floor((expires - Date.now()) / 1000)),
    };
  }
  subject(id, now = Date.now()) {
    const session = this.#sessions.get(id);
    if (session && now < session.expires) return session.subject;
    this.#sessions.delete(id);
    return undefined;
  }
  logout(id) {
    this.#sessions.delete(id);
  }
  expire() {
    for (const id of this.#sessions.keys()) this.subject(id);
  }
}
