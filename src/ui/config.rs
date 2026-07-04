use crate::state::{
    CollectionMode, CsiDeliveryMode, CsiForm, DeviceAction, DeviceState, Ht40Mode, OutputMode,
    PHY_RATES, WiFiMode, WifiProtocol,
};

/// Render the configuration view for one device.
///
/// Pushes per-device actions into `actions`; the caller addresses them to the
/// selected device id after rendering.
pub fn render(ui: &mut egui::Ui, device: &mut DeviceState, actions: &mut Vec<DeviceAction>) {
    ui.add(
        egui::Label::new(format!("Configuration — {}", device.id))
            .wrap(),
    );
    ui.add_space(8.0);
    render_body(ui, device, actions);
}

fn render_body(ui: &mut egui::Ui, device: &mut DeviceState, actions: &mut Vec<DeviceAction>) {
    let device_id = device.id.clone();
    let supports_v07 = device.supports_v07_modes();
    let supports_unsolicited = device.supports_unsolicited();
    let forms = &mut device.forms;
    let field_width = text_field_width(ui, 140.0);

    section_header(ui, "Wi-Fi", |ui| {
        form_row(ui, "Mode", |ui| {
            wifi_mode_picker(ui, &device_id, supports_v07, &mut forms.wifi.mode);
        });

        if forms.wifi.mode.requires_v07() && !supports_v07 {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Selected mode requires esp-csi-cli-rs ≥ 0.7.0 on this device.",
            );
        }

        if matches!(forms.wifi.mode, WiFiMode::Station) {
            form_row(ui, "STA SSID", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut forms.wifi.sta_ssid).desired_width(field_width),
                );
            });

            form_row(ui, "STA Password", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut forms.wifi.sta_password)
                        .password(true)
                        .desired_width(field_width),
                );
            });
        }

        if matches!(forms.wifi.mode, WiFiMode::WifiAp) {
            form_row(ui, "AP SSID", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut forms.wifi.ap_ssid).desired_width(field_width),
                );
            });

            form_row(ui, "AP Password", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut forms.wifi.ap_password)
                        .password(true)
                        .desired_width(field_width),
                );
            });

            ui.checkbox(&mut forms.wifi.ap_dhcp, "AP DHCP (built-in single-lease server)");
        }

        if forms.wifi.mode.allows_channel() {
            form_row(ui, "Channel", |ui| {
                ui.add(egui::TextEdit::singleline(&mut forms.wifi.channel).desired_width(80.0));
            });
        } else {
            ui.add(
                egui::Label::new(
                    "Channel — inherited from the associated AP (not configurable in station mode).",
                )
                .wrap(),
            );
            ui.add_space(6.0);
        }

        if forms.wifi.mode.is_esp_now() {
            form_row(ui, "Peer MAC", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut forms.wifi.peer_mac)
                        .hint_text("auto")
                        .desired_width(field_width),
                );
            });

            form_row(ui, "HT40 secondary", |ui| {
                ht40_picker(ui, &mut forms.wifi.ht40);
            });

            ui.add_space(4.0);
            ui.add(
                egui::Label::new(
                    "Peer MAC / HT40 apply to all ESP-NOW modes; empty MAC = auto.",
                )
                .wrap(),
            );
        }

        ui.add_space(8.0);
        if ui.button("Apply Wi-Fi Config").clicked() {
            actions.push(DeviceAction::SetWifi(forms.wifi.clone()));
        }

        ui.add_space(12.0);
        form_row(ui, "PHY Protocol", |ui| {
            protocol_picker(ui, &mut forms.protocol);
            if ui.button("Apply Protocol").clicked() {
                actions.push(DeviceAction::SetProtocol(forms.protocol));
            }
        });
        ui.add(
            egui::Label::new(
                "Applied at the start of each run. Default lr (Long-Range) is proprietary; \
                 use n / ax to associate with a standard AP in station mode.",
            )
            .wrap(),
        );
    });

    section_header(ui, "Traffic", |ui| {
        form_row(ui, "Frequency (Hz)", |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut forms.traffic.frequency_hz).desired_width(120.0),
            );
            if ui.button("Apply Traffic Config").clicked() {
                actions.push(DeviceAction::SetTraffic(forms.traffic.clone()));
            }
        });
        ui.add(
            egui::Label::new(
                "For AP + station lab pairs: AP floods at 1000 Hz, station stays \
                 receive-only (frequency 0).",
            )
            .wrap(),
        );

        // The flood-kind toggle only exists for the WiFi infra modes: ESP-NOW
        // modes transmit their own frames and sniffers generate no traffic.
        if matches!(forms.wifi.mode, WiFiMode::WifiAp | WiFiMode::Station) {
            let traffic_on = forms
                .traffic
                .frequency_hz
                .trim()
                .parse::<u64>()
                .is_ok_and(|hz| hz > 0);
            ui.add_space(4.0);
            // Moot while traffic generation is off (frequency 0/blank), and
            // unsupported (whole set-traffic command rejected!) below 0.7.1.
            ui.add_enabled_ui(traffic_on && supports_unsolicited, |ui| {
                ui.checkbox(
                    &mut forms.traffic.unsolicited,
                    "Unsolicited echo-reply flood (one-directional)",
                );
            });
            if !supports_unsolicited {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Unsolicited flood requires esp-csi-cli-rs ≥ 0.7.1 on this device — \
                     reflash to use it (the flag is not sent to older firmware).",
                );
            }
            ui.add(
                egui::Label::new(
                    "Floods unsolicited echo replies the peer silently ignores: no reply \
                     contention, stable offered rate — but this device captures no CSI \
                     back from the peer; collect on the peer instead.",
                )
                .wrap(),
            );
            if traffic_on
                && forms.traffic.unsolicited
                && matches!(forms.collection_mode, CollectionMode::Collector)
            {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Unsolicited flood + Collector: this node will capture ~no CSI. \
                     Set Collector on the peer, or disable the unsolicited flood.",
                );
            }
        }
    });

    section_header(ui, "CSI Flags", |ui| {
        render_csi_flags(ui, &mut forms.csi);

        ui.add_space(8.0);
        form_row(ui, "csi_he_stbc (u32)", |ui| {
            ui.add(egui::TextEdit::singleline(&mut forms.csi.csi_he_stbc).desired_width(80.0));
        });
        form_row(ui, "val_scale_cfg (u32)", |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut forms.csi.val_scale_cfg).desired_width(80.0),
            );
        });

        ui.add_space(8.0);
        if ui.button("Apply CSI Config").clicked() {
            actions.push(DeviceAction::SetCsi(forms.csi.clone()));
        }
        if ui.button("Apply HE20 preset").clicked() {
            actions.push(DeviceAction::SetCsiPreset("he20"));
        }
    });

    section_header(ui, "PHY Rate", |ui| {
        form_row(ui, "Rate", |ui| {
            phy_rate_picker(ui, &mut forms.phy_rate.rate);
            if ui.button("Apply Rate").clicked() {
                actions.push(DeviceAction::SetPhyRate(forms.phy_rate.clone()));
            }
        });
        ui.add(
            egui::Label::new(
                "Honored by all ESP-NOW modes (incl. fast simplex) and wifi-ap/sniffer; \
                 ignored by station.",
            )
            .wrap(),
        );
    });

    section_header(ui, "IO Tasks", |ui| {
        ui.checkbox(&mut forms.io_tasks.tx, "TX (traffic generation)");
        ui.add_space(4.0);
        ui.checkbox(&mut forms.io_tasks.rx, "RX (CSI capture)");
        ui.add_space(8.0);
        if ui.button("Apply IO Tasks").clicked() {
            actions.push(DeviceAction::SetIoTasks(forms.io_tasks.clone()));
        }
    });

    section_header(ui, "CSI Delivery", |ui| {
        form_row(ui, "Mode", |ui| {
            csi_delivery_picker(ui, &mut forms.csi_delivery.mode);
        });
        ui.checkbox(&mut forms.csi_delivery.logging, "inline UART logging");
        ui.add_space(8.0);
        if ui.button("Apply CSI Delivery").clicked() {
            actions.push(DeviceAction::SetCsiDelivery(forms.csi_delivery.clone()));
        }
    });

    section_header(ui, "Collection & Output", |ui| {
        form_row(ui, "Collection Mode", |ui| {
            collection_mode_picker(ui, &mut forms.collection_mode);
            if ui.button("Apply").clicked() {
                actions.push(DeviceAction::SetCollectionMode(forms.collection_mode));
            }
        });

        form_row(ui, "Output Mode", |ui| {
            output_mode_picker(ui, &mut forms.output_mode);
            if ui.button("Apply").clicked() {
                actions.push(DeviceAction::SetOutputMode(forms.output_mode));
            }
        });
    });

    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("Reset Config Defaults").clicked() {
            actions.push(DeviceAction::ResetConfig);
        }
        if ui.button("Refresh Config").clicked() {
            actions.push(DeviceAction::FetchConfig);
        }
    });
}

/// Collapsing section, open by default, with spacing below.
fn section_header(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            ui.add_space(6.0);
            body(ui);
        });
    ui.add_space(12.0);
}

/// One labeled form row; wraps when the panel is narrower than label + controls.
fn form_row(ui: &mut egui::Ui, label: &str, widgets: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_wrapped(|ui| {
        ui.label(label);
        ui.add_space(8.0);
        widgets(ui);
    });
    ui.add_space(6.0);
}

/// Width for text inputs: fill remaining panel space without forcing overflow.
fn text_field_width(ui: &egui::Ui, label_reserve: f32) -> f32 {
    (ui.available_width() - label_reserve - 16.0)
        .clamp(80.0, 480.0)
}

/// CSI disable flags — explicit columns sized to the panel (Grid overflows here).
fn render_csi_flags(ui: &mut egui::Ui, csi: &mut CsiForm) {
    let panel_w = ui.available_width();
    let two_cols = panel_w >= 360.0;

    if two_cols {
        let col_w = ((panel_w - 12.0) / 2.0).max(120.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width(col_w);
                csi_flag_column_a(ui, csi);
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.set_max_width(col_w);
                csi_flag_column_b(ui, csi);
            });
        });
    } else {
        ui.vertical(|ui| {
            ui.set_max_width(panel_w);
            csi_flag_column_a(ui, csi);
            csi_flag_column_b(ui, csi);
        });
    }
}

fn csi_flag_column_a(ui: &mut egui::Ui, csi: &mut CsiForm) {
    ui.checkbox(&mut csi.lltf, "lltf");
    ui.checkbox(&mut csi.htltf, "htltf");
    ui.checkbox(&mut csi.stbc_htltf, "stbc_htltf");
    ui.checkbox(&mut csi.ltf_merge, "ltf_merge");
    ui.checkbox(&mut csi.csi, "csi");
    ui.checkbox(&mut csi.csi_legacy, "csi_legacy");
}

fn csi_flag_column_b(ui: &mut egui::Ui, csi: &mut CsiForm) {
    ui.checkbox(&mut csi.csi_ht20, "csi_ht20");
    ui.checkbox(&mut csi.csi_ht40, "csi_ht40");
    ui.checkbox(&mut csi.csi_su, "csi_su");
    ui.checkbox(&mut csi.csi_mu, "csi_mu");
    ui.checkbox(&mut csi.csi_dcm, "csi_dcm");
    ui.checkbox(&mut csi.csi_beamformed, "csi_beamformed");
    ui.checkbox(&mut csi.dump_ack, "dump_ack");
    ui.checkbox(&mut csi.csi_force_lltf, "csi_force_lltf (C5)");
    ui.checkbox(&mut csi.csi_vht, "csi_vht (C5)");
}

fn wifi_mode_picker(ui: &mut egui::Ui, device_id: &str, supports_v07: bool, mode: &mut WiFiMode) {
    egui::ComboBox::from_id_salt(format!("wifi_mode_combo_{device_id}"))
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, WiFiMode::Station, "station");
            ui.selectable_value(mode, WiFiMode::Sniffer, "sniffer");
            ui.add_enabled_ui(supports_v07, |ui| {
                ui.selectable_value(mode, WiFiMode::WifiAp, "wifi-ap (≥ 0.7.0)");
            });
            ui.selectable_value(mode, WiFiMode::EspNowCentral, "esp-now-central");
            ui.selectable_value(mode, WiFiMode::EspNowPeripheral, "esp-now-peripheral");
            ui.add_enabled_ui(supports_v07, |ui| {
                ui.selectable_value(
                    mode,
                    WiFiMode::EspNowFastCollector,
                    "esp-now-fast-collector (≥ 0.7.0)",
                );
                ui.selectable_value(
                    mode,
                    WiFiMode::EspNowFastSource,
                    "esp-now-fast-source (≥ 0.7.0)",
                );
            });
        });
}

fn collection_mode_picker(ui: &mut egui::Ui, mode: &mut CollectionMode) {
    egui::ComboBox::from_id_salt("collection_mode_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, CollectionMode::Collector, "collector");
            ui.selectable_value(mode, CollectionMode::Listener, "listener");
        });
}

fn ht40_picker(ui: &mut egui::Ui, mode: &mut Ht40Mode) {
    egui::ComboBox::from_id_salt("ht40_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, Ht40Mode::None, "none");
            ui.selectable_value(mode, Ht40Mode::Above, "above");
            ui.selectable_value(mode, Ht40Mode::Below, "below");
        });
}

fn output_mode_picker(ui: &mut egui::Ui, mode: &mut OutputMode) {
    egui::ComboBox::from_id_salt("output_mode_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, OutputMode::Stream, "stream");
            ui.selectable_value(mode, OutputMode::Dump, "dump");
            ui.selectable_value(mode, OutputMode::Both, "both");
        });
}

fn csi_delivery_picker(ui: &mut egui::Ui, mode: &mut CsiDeliveryMode) {
    egui::ComboBox::from_id_salt("csi_delivery_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, CsiDeliveryMode::Off, "off");
            ui.selectable_value(mode, CsiDeliveryMode::Callback, "callback");
            ui.selectable_value(mode, CsiDeliveryMode::Async, "async");
            ui.selectable_value(mode, CsiDeliveryMode::Raw, "raw");
        });
}

fn protocol_picker(ui: &mut egui::Ui, protocol: &mut WifiProtocol) {
    egui::ComboBox::from_id_salt("protocol_combo")
        .selected_text(protocol.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(protocol, WifiProtocol::B, "b");
            ui.selectable_value(protocol, WifiProtocol::G, "g");
            ui.selectable_value(protocol, WifiProtocol::N, "n");
            ui.selectable_value(protocol, WifiProtocol::Lr, "lr");
            ui.selectable_value(protocol, WifiProtocol::A, "a");
            ui.selectable_value(protocol, WifiProtocol::Ac, "ac");
            ui.selectable_value(protocol, WifiProtocol::Ax, "ax");
        });
}

fn phy_rate_picker(ui: &mut egui::Ui, rate: &mut String) {
    egui::ComboBox::from_id_salt("phy_rate_combo")
        .selected_text(rate.as_str())
        .show_ui(ui, |ui| {
            for option in PHY_RATES {
                ui.selectable_value(rate, (*option).to_owned(), *option);
            }
        });
}
