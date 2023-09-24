use std::collections::HashSet;

pub static TRACKERS: LazyCell<HashSet<&'static str>> = LazyCell::new(|| {
    vec![
        // Google Urchin Tracking Module
        "utm_source",
        "utm_medium",
        "utm_term",
        "utm_campaign",
        "utm_content",
        "utm_name",
        "utm_cid",
        "utm_reader",
        "utm_viz_id",
        "utm_pubreferrer",
        "utm_swu",
        // Adobe Omniture SiteCatalyst
        "ICID",
        "icid",
        // Hubspot
        "_hsenc",
        "_hsmi",
        // Marketo
        "mkt_tok",
        // MailChimp
        // https://developer.mailchimp.com/documentation/mailchimp/guides/getting-started-with-ecommerce/
        "mc_cid",
        "mc_eid",
        // comScore Digital Analytix?
        // http://www.about-digitalanalytics.com/comscore-digital-analytix-url-campaign-generator
        "ns_source",
        "ns_mchannel",
        "ns_campaign",
        "ns_linkname",
        "ns_fee",
        // Simple Reach
        "sr_share",
        // Vero
        "vero_conv",
        "vero_id",
        // Facebook Click Identifier
        // http://thisinterestsme.com/facebook-fbclid-parameter/
        "fbclid",
        // Instagram Share Identifier
        "igshid",
        "srcid",
        // Google Click Identifier
        "gclid",
        // Some other Google Click thing
        "ocid",
        // Unknown
        "ncid",
        // Unknown
        "nr_email_referer",
        // Generic-ish. Facebook, Product Hunt and others
        "ref",
        // Alibaba-family super position model tracker:
        // https://github.com/newhouse/url-tracking-stripper/issues/38
        "spm",
    ]
    .into_iter()
    .collect()
});
