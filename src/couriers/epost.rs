use anyhow::{anyhow, Result};
use nipper::Document;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::get_html_string;

use super::{Courier, DeliveryStatus, StateType, TrackingStatus};

pub const URL: &str = "https://service.epost.go.kr/trace.RetrieveDomRigiTraceList.comm";
pub const ID: &str = "kr.epost";
pub const NAME: &str = "우체국";

static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^(\d{13})$"#).unwrap());

pub fn validate(courier: &Courier) -> Result<()> {
    if !REGEX.is_match(&courier.tracking_number) {
        Err(anyhow!("운송장번호 13자리를 입력해주세요."))
    } else {
        Ok(())
    }
}

pub fn state_from(status: &str) -> StateType {
    use StateType::*;
    if status == "접수" {
        InformationReceived
    } else if status == "발송" || status == "도착" {
        InTransit
    } else if status.contains("배달준비") {
        OutForDelivery
    } else if status.contains("배달완료") {
        Delivered
    } else {
        Unknown
    }
}

pub async fn track(courier: &Courier) -> Result<DeliveryStatus> {
    let response = surf::post(URL)
        .body(format!("sid1={}&displayHeader=N", courier.tracking_number))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.90 Safari/537.36")
        .recv_string()
        .await
        .map_err(|err| anyhow!(err))?;
    let document = Document::from(&response);

    if document
        .select("#print > table > tbody > tr:nth-child(2) > td")
        .exists()
    {
        return Err(anyhow!(
            "{} {} 운송장 번호로 조회된 결과가 없습니다.",
            NAME,
            courier.tracking_number
        ));
    }

    let tracking_number = get_html_string!(document, "#print > table > tbody > tr > th");
    let sender = get_html_string!(document, "#print > table > tbody > tr > td:nth-child(2)");
    let receiver = get_html_string!(document, "#print > table > tbody > tr > td:nth-child(3)");

    let mut tracks: Vec<TrackingStatus> = Vec::new();

    for element in document.select("#processTable > tbody > tr").iter() {
        let status = get_html_string!(element, "td:nth-child(4)");
        tracks.push(TrackingStatus {
            state: state_from(&status),
            time: format!(
                "{} {}",
                get_html_string!(element, "td:nth-child(1)"),
                get_html_string!(element, "td:nth-child(2)")
            ),
            location: Some(get_html_string!(element, "td:nth-child(3)")),
            status,
            message: None,
        });
    }

    Ok(DeliveryStatus {
        id: ID.to_string(),
        name: NAME.to_string(),
        tracking_number,
        sender: Some(sender),
        receiver: Some(receiver),
        product: None,
        tracks: Some(tracks),
    })
}
