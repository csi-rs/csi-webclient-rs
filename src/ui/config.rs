use crate::state::{
    AppState, CollectionMode, LogMode, OutputMode, UserIntent, WiFiMode,
};

/// Render the configuration view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Configuration");
    ui.separator();

    ui.collapsing("Wi-Fi", |ui| {
        ui.horizontal(|ui| {
            ui.label("Mode");
            mode_picker(ui, &mut state.persistent.wifi.mode);
        });

        ui.horizontal(|ui| {
            ui.label("STA SSID");
            ui.text_edit_singleline(&mut state.persistent.wifi.sta_ssid);
        });

        ui.horizontal(|ui| {
            ui.label("STA Password");
            ui.add(egui::TextEdit::singleline(&mut state.persistent.wifi.sta_password).password(true));
        });

        ui.horizontal(|ui| {
            ui.label("Channel");
            ui.text_edit_singleline(&mut state.persistent.wifi.channel);
        });

        if ui.button("Apply Wi-Fi Config").clicked() {
            state.push_intent(UserIntent::SetWifi(state.persistent.wifi.clone()));
        }
    });

    ui.separator();

    ui.collapsing("Traffic", |ui| {
        ui.horizontal(|ui| {
            ui.label("Frequency (Hz)");
            ui.text_edit_singleline(&mut state.persistent.traffic.frequency_hz);
            if ui.button("Apply Traffic Config").clicked() {
                state.push_intent(UserIntent::SetTraffic(state.persistent.traffic.clone()));
            }
        });
    });

    ui.separator();

    ui.collapsing("CSI Flags", |ui| {
        ui.columns(2, |columns| {
            columns[0].checkbox(&mut state.persistent.csi.disable_lltf, "disable_lltf");
            columns[0].checkbox(&mut state.persistent.csi.disable_htltf, "disable_htltf");
            columns[0].checkbox(&mut state.persistent.csi.disable_stbc_htltf, "disable_stbc_htltf");
            columns[0].checkbox(&mut state.persistent.csi.disable_ltf_merge, "disable_ltf_merge");
            columns[0].checkbox(&mut state.persistent.csi.disable_csi, "disable_csi");
            columns[0].checkbox(&mut state.persistent.csi.disable_csi_legacy, "disable_csi_legacy");
            columns[1].checkbox(&mut state.persistent.csi.disable_csi_ht20, "disable_csi_ht20");
            columns[1].checkbox(&mut state.persistent.csi.disable_csi_ht40, "disable_csi_ht40");
            columns[1].checkbox(&mut state.persistent.csi.disable_csi_su, "disable_csi_su");
            columns[1].checkbox(&mut state.persistent.csi.disable_csi_mu, "disable_csi_mu");
            columns[1].checkbox(&mut state.persistent.csi.disable_csi_dcm, "disable_csi_dcm");
            columns[1].checkbox(&mut state.persistent.csi.disable_csi_beamformed, "disable_csi_beamformed");
        });

        ui.horizontal(|ui| {
            ui.label("csi_he_stbc (u8)");
            ui.text_edit_singleline(&mut state.persistent.csi.csi_he_stbc);
        });

        ui.horizontal(|ui| {
            ui.label("val_scale_cfg (u8)");
            ui.text_edit_singleline(&mut state.persistent.csi.val_scale_cfg);
        });

        if ui.button("Apply CSI Config").clicked() {
            state.push_intent(UserIntent::SetCsi(state.persistent.csi.clone()));
        }
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Collection Mode");
        collection_mode_picker(ui, &mut state.persistent.collection_mode);
        if ui.button("Apply").clicked() {
            state.push_intent(UserIntent::SetCollectionMode(state.persistent.collection_mode));
        }
    });

    ui.horizontal(|ui| {
        ui.label("Log Mode");
        log_mode_picker(ui, &mut state.persistent.log_mode);
        if ui.button("Apply").clicked() {
            state.push_intent(UserIntent::SetLogMode(state.persistent.log_mode));
        }
    });

    ui.horizontal(|ui| {
        ui.label("Output Mode");
        output_mode_picker(ui, &mut state.persistent.output_mode);
        if ui.button("Apply").clicked() {
            state.push_intent(UserIntent::SetOutputMode(state.persistent.output_mode));
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Reset Config Defaults").clicked() {
            state.push_intent(UserIntent::ResetConfig);
        }

        if ui.button("Refresh Config").clicked() {
            state.push_intent(UserIntent::FetchConfig);
        }
    });
}

fn mode_picker(ui: &mut egui::Ui, mode: &mut WiFiMode) {
    egui::ComboBox::from_id_salt("wifi_mode_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, WiFiMode::Sta, "sta");
            ui.selectable_value(mode, WiFiMode::Monitor, "monitor");
            ui.selectable_value(mode, WiFiMode::Sniffer, "sniffer");
        });
}

/// Render collection mode selector.
fn collection_mode_picker(ui: &mut egui::Ui, mode: &mut CollectionMode) {
    egui::ComboBox::from_id_salt("collection_mode_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, CollectionMode::Collector, "collector");
            ui.selectable_value(mode, CollectionMode::Listener, "listener");
        });
}

/// Render log mode selector.
fn log_mode_picker(ui: &mut egui::Ui, mode: &mut LogMode) {
    egui::ComboBox::from_id_salt("log_mode_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, LogMode::Text, "text");
            ui.selectable_value(mode, LogMode::ArrayList, "array-list");
            ui.selectable_value(mode, LogMode::Serialized, "serialized");
        });
}

/// Render output mode selector.
fn output_mode_picker(ui: &mut egui::Ui, mode: &mut OutputMode) {
    egui::ComboBox::from_id_salt("output_mode_combo")
        .selected_text(mode.as_api_value())
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, OutputMode::Stream, "stream");
            ui.selectable_value(mode, OutputMode::Dump, "dump");
            ui.selectable_value(mode, OutputMode::Both, "both");
        });
}
