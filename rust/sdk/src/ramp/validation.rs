use super::*;

pub(crate) fn location(location: &RampLocation) -> Result<(), &'static str> {
    if location.country_code.trim().is_empty() {
        return Err("country code is required");
    }
    if location
        .subdivision_code
        .as_ref()
        .is_some_and(|code| code.trim().is_empty())
    {
        return Err("subdivision code must not be empty");
    }
    Ok(())
}

pub(crate) fn normalize_order_request(order: &mut RampOrderRequest) -> Result<(), &'static str> {
    let (currency, amount) = match order {
        RampOrderRequest::Buy { spend, .. } => (&mut spend.currency_code, spend.minor_units),
        RampOrderRequest::Sell {
            sell,
            receive_fiat_currency_code,
        } => (receive_fiat_currency_code, sell.base_units),
    };
    if currency.trim().is_empty() || amount == 0 {
        return Err("currency and positive amount are required");
    }
    *currency = currency.trim().to_ascii_uppercase();
    Ok(())
}

fn buy_quote(quote: &RampBuyQuote) -> Result<(), &'static str> {
    if quote.spend.currency_code.is_empty()
        || quote.spend.currency_code != quote.total_fee.currency_code
        || quote.exchange_rate.is_empty()
    {
        return Err("incomplete Ramp buy quote");
    }
    Ok(())
}

fn sell_quote(quote: &RampSellQuote) -> Result<(), &'static str> {
    if quote.receive.currency_code.is_empty()
        || quote.receive.currency_code != quote.total_fee.currency_code
        || quote.exchange_rate.is_empty()
    {
        return Err("incomplete Ramp sell quote");
    }
    Ok(())
}

pub(crate) fn quote(quote: &RampQuote, request: &RampOrderRequest) -> Result<(), &'static str> {
    if quote.route.provider.is_empty() || quote.route.payment_method.is_empty() {
        return Err("incomplete Ramp route");
    }
    match (&quote.details, request) {
        (RampQuoteDetails::Buy(quote), RampOrderRequest::Buy { spend, receive }) => {
            buy_quote(quote)?;
            if quote.spend.currency_code != spend.currency_code || quote.receive.asset != *receive {
                return Err("Ramp quote currency or asset mismatch");
            }
            Ok(())
        }
        (
            RampQuoteDetails::Sell(quote),
            RampOrderRequest::Sell {
                sell,
                receive_fiat_currency_code,
            },
        ) => {
            sell_quote(quote)?;
            if quote.sell.asset != sell.asset
                || &quote.receive.currency_code != receive_fiat_currency_code
            {
                return Err("Ramp quote currency or asset mismatch");
            }
            Ok(())
        }
        _ => Err("Ramp quote direction mismatch"),
    }
}
pub(crate) fn order(order: &RampOrder) -> Result<(), &'static str> {
    match order {
        RampOrder::Buy(order) => buy_quote(&order.quote),
        RampOrder::Sell(order) => {
            sell_quote(&order.quote)?;
            if order
                .deposit
                .as_ref()
                .is_some_and(|deposit| deposit.amount.base_units == 0)
            {
                return Err("Ramp deposit amount must be positive");
            }
            if order
                .deposit
                .as_ref()
                .is_some_and(|deposit| deposit.amount.asset != order.quote.sell.asset)
            {
                return Err("Ramp deposit asset mismatch");
            }
            Ok(())
        }
    }
}

pub(crate) fn created_order(
    value: &RampOrder,
    request: &RampOrderRequest,
) -> Result<(), &'static str> {
    order(value)?;
    let matches_request = match (value, request) {
        (RampOrder::Buy(order), RampOrderRequest::Buy { spend, receive }) => {
            order.quote.spend.currency_code == spend.currency_code
                && order.quote.receive.asset == *receive
        }
        (
            RampOrder::Sell(order),
            RampOrderRequest::Sell {
                sell,
                receive_fiat_currency_code,
            },
        ) => {
            order.quote.sell.asset == sell.asset
                && &order.quote.receive.currency_code == receive_fiat_currency_code
        }
        _ => return Err("Ramp order direction mismatch"),
    };
    if !matches_request {
        return Err("Ramp order currency or asset mismatch");
    }
    Ok(())
}
