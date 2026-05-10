use crate::state::{
    AppState, CollectionMode, CsiDeliveryMode, LogMode, OutputMode, PHY_RATES, UserIntent, WiFiMode,
};

/// Render the configuration view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Configuration");
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_body(ui, state);
        });
}

fn render_body(ui: &mut egui::Ui, state: &mut AppState) {
    ui.collapsing("Wi-Fi", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Mode");
            wifi_mode_picker(ui, &mut state.persistent.wifi.mode);
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("STA SSID");
            ui.add(
                egui::TextEdit::singleline(&mut state.persistent.wifi.sta_ssid)
                    .desired_width(220.0),
            );
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("STA Password");
            ui.add(
                egui::TextEdit::singleline(&mut state.persistent.wifi.sta_password)
                    .password(true)
                    .desired_width(220.0),
            );
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Channel");
            ui.add(
                egui::TextEdit::singleline(&mut state.persistent.wifi.channel)
                    .desired_width(60.0),
            );
        });

        if ui.button("Apply Wi-Fi Config").clicked() {
            state.push_intent(UserIntent::SetWifi(state.persistent.wifi.clone()));
        }
    });

    ui.separator();

    ui.collapsing("Traffic", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Frequency (Hz)");
            ui.add(
                egui::TextEdit::singleline(&mut state.persistent.traffic.frequency_hz)
                    .desired_width(100.0),
            );
            if ui.button("Apply Traffic Config").clicked() {
                state.push_intent(UserIntent::SetTraffic(state.persistent.traffic.clone()));
            }
        });
    });

    ui.separator();

    ui.collapsing("CSI Flags", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut state.persistent.csi.disable_lltf, "disable_lltf");
            ui.checkbox(&mut state.persistent.csi.disable_htltf, "disable_htltf");
            ui.checkbox(
                &mut state.persistent.csi.disable_stbc_htltf,
                "disable_stbc_htltf",
            );
            ui.checkbox(
                &mut state.persistent.csi.disable_ltf_merge,
                "disable_ltf_merge",
            );
            ui.checkbox(&mut state.persistent.csi.disable_csi, "disable_csi");
            ui.checkbox(
                &mut state.persistent.csi.disable_csi_legacy,
                "disable_csi_legacy",
            );
            ui.checkbox(
                &mut state.persistent.csi.disable_csi_ht20,
                "disable_csi_ht20",
            );
            ui.checkbox(
                &mut state.persistent.csi.disable_csi_ht40,
                "disable_csi_ht40",
            );
            ui.checkbox(&mut state.persistent.csi.disable_csi_su, "disable_csi_su");
            ui.checkbox(&mut state.persistent.csi.disable_csi_mu, "disable_csi_mu");
            ui.checkbox(&mut state.persistent.csi.disable_csi_dcm, "disable_csi_dcm");
            ui.checkbox(
                &mut state.persistent.csi.disable_csi_beamformed,
                "disable_csi_beamformed",
            );
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("csi_he_stbc (u32)");
            ui.add(
                egui::TextEdit::singleline(&mut state.persistent.csi.csi_he_stbc)
                    .desired_width(80.0),
            );
            ui.label("val_scale_cfg (u32)");
            ui.add(
                egui::TextEdit::singleline(&mut state.persistent.csi.val_scale_cfg)
                    .desired_width(80.0),
            );
        });

        if ui.button("Apply CSI Config").clicked() {
            state.push_intent(UserIntent::SetCsi(state.persistent.csi.clone()));
        }
    });

    ui.separator();

    ui.collapsing("PHY Rate (ESP-NOW only)", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Rate");
            phy_rate_picker(ui, &mut state.persistent.phy_rate.rate);
            if ui.button("Apply Rate").clicked() {
                state.push_intent(UserIntent::SetPhyRate(state.persistent.phy_rate.clone()));
            }
        });
        ui.add(
            egui::Label::new(
                "Honored by esp-now-central / esp-now-peripheral; ignored by station/sniffer.",
            )
            .wrap(),
        );
    });

    ui.separator();

    ui.collapsing("IO Tasks", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut state.persistent.io_tasks.tx, "TX (traffic generation)");
            ui.checkbox(&mut state.persistent.io_tasks.rx, "RX (CSI capture)");
        });
        if ui.button("Apply IO Tasks").clicked() {
            state.push_intent(UserIntent::SetIoTasks(state.persistent.io_tasks.clone()));
        }
    });

    ui.separator();

    ui.collapsing("CSI Delivery", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Mode");
            csi_delivery_picker(ui, &mut state.persistent.csi_delivery.mode);
            ui.checkbox(
                &mut state.persistent.csi_delivery.logging,
                "inline UART logging",
            );
        });
        if ui.button("Apply CSI Delivery").clicked() {
            state.push_intent(UserIntent::SetCsiDelivery(
                state.persistent.csi_delivery.clone(),
            ));
        }
    });

    ui.separator();

    ui.horizontal_wrapped(|ui| {
        ui.label("Collection Mode");
        collection_mode_picker(ui, &mut state.persistent.collection_mode);
        if ui.button("Apply").clicked() {
            state.push_intent(UserIntent::SetCollectionMode(
                state.persistent.collection_mode,
            ));
        }
    });

    ui.horizontal_wrapped(|ui| {
        ui.label("Log Mode");
        log_mode_picker(ui, &mut state.persistent.log_mode);
        if ui.button("Apply").clicked() {
            state.push_intent(UserIntent::SetLogMode(state.persistent.log_mode));
        }
    });

    ui.horizontal_wrapped(|ui| {
        ui.label("Output Mode");
        output_mode_picker(ui, &mut state.persistent.output_mode);
        if ui.button("Apply").clicked() {
            state.push_intent(UserIntent::SetOutputMode(state.persistent.output_mode));
        }
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button("Reset Config Defaults").clicked() {
            state.push_intent(UserIntent::ResetConfig);
        }

        if ui.button("Refresh Config").clicked() {
            state.push_intent(UserIntent::FetchConfig);
        }
    });
}

fn wifi_mode_picker(ui: &mut egui::Ui, mode: &mut WiFiMode) {
    egui::ComboBox::from_id_salt("wifi_mode_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, WiFiMode::Station, "station");
            ui.selectable_value(mode, WiFiMode::Sniffer, "sniffer");
            ui.selectable_value(mode, WiFiMode::EspNowCentral, "esp-now-central");
            ui.selectable_value(mode, WiFiMode::EspNowPeripheral, "esp-now-peripheral");
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

fn log_mode_picker(ui: &mut egui::Ui, mode: &mut LogMode) {
    egui::ComboBox::from_id_salt("log_mode_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, LogMode::Text, "text");
            ui.selectable_value(mode, LogMode::ArrayList, "array-list");
            ui.selectable_value(mode, LogMode::Serialized, "serialized");
            ui.selectable_value(mode, LogMode::EspCsiTool, "esp-csi-tool");
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
