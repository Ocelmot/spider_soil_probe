use spider_client::link::{SpiderId2048, message::{UiPageManager, UiPath, UiElement, UiElementKind}};

pub fn init_ui(id: SpiderId2048) -> UiPageManager {
    let mut ui_page = UiPageManager::new(id, "Sluice");
        
    // Initialise the UIPage
    let mut root = ui_page
        .get_element_mut(&UiPath::root())
        .expect("all pages have a root");
    root.set_kind(UiElementKind::Rows);

    // Status Section
    root.append_child({
        let mut element = UiElement::new(UiElementKind::Header);
        element.set_text("Status");
        element
    });
    root.append_child({
        let mut element = UiElement::new(UiElementKind::Columns);
        element.append_child(UiElement::from_string("Temperature: "));
        element.append_child({
            let mut element = UiElement::from_string("-");
            element.set_id("temp");
            element
        });
        element.append_child(UiElement::new(UiElementKind::Spacer));
        element
    });
    root.append_child({
        let mut element = UiElement::new(UiElementKind::Columns);
        element.append_child(UiElement::from_string("Moisture: "));
        element.append_child({
            let mut element = UiElement::from_string("-");
            element.set_id("water");
            element
        });
        element.append_child(UiElement::new(UiElementKind::Spacer));
        element
    });
    root.append_child({
        let mut element = UiElement::new(UiElementKind::Columns);
        element.append_child(UiElement::from_string("Remaining time: "));
        element.append_child({
            let mut element = UiElement::from_string("0:00");
            element.set_id("remaining_time");
            element
        });
        element.append_child(UiElement::new(UiElementKind::Spacer));
        element
    });

    root.append_child({
        let mut control_row = UiElement::new(UiElementKind::Columns);
        control_row.append_child({
            let mut button = UiElement::new(UiElementKind::Button);
            button.set_text("+5 Min");
            button.set_id("inc_5_min");
            button
        });
        control_row.append_child({
            let mut button = UiElement::new(UiElementKind::Button);
            button.set_text("+10 Min");
            button.set_id("inc_10_min");
            button
        });
        control_row.append_child({
            let mut button = UiElement::new(UiElementKind::Button);
            button.set_text("+1 Day");
            button.set_id("inc_1_day");
            button
        });
        control_row.append_child({
            let mut button = UiElement::new(UiElementKind::Button);
            button.set_text("Off");
            button.set_id("clear_timer");
            button
        });

        control_row.append_child(UiElement::new(UiElementKind::Spacer));
        control_row
    });

    // Schedule Section
    root.append_child({
        let mut element = UiElement::new(UiElementKind::Header);
        element.set_text("Schedule");
        element
    });
    root.append_child({
        let mut element = UiElement::new(UiElementKind::Columns);
        element.append_child(UiElement::from_string("Current Time:"));
        element.append_child({
            let mut element = UiElement::from_string("00:00");
            element.set_id("clock_time");
            element
        });
        element.append_child(UiElement::new(UiElementKind::Spacer));
        element
    });

    // Config Section
    root.append_child({
        let mut element = UiElement::new(UiElementKind::Header);
        element.set_text("Config");
        element
    });
    root.append_child({
        let mut config_row = UiElement::new(UiElementKind::Columns);
        config_row.append_child(UiElement::from_string("Button Led"));
        config_row.append_child(UiElement::new(UiElementKind::Spacer));
        config_row.append_child({
            let mut button = UiElement::new(UiElementKind::Button);
            button.set_text("Disable");
            button.set_id("toggle_button_led");
            button
        });
        config_row
    });
    root.append_child({
        let mut config_row = UiElement::new(UiElementKind::Columns);
        config_row.append_child(UiElement::from_string("Temperature Unit"));
        config_row.append_child(UiElement::new(UiElementKind::Spacer));
        config_row.append_child({
            let mut button = UiElement::new(UiElementKind::Button);
            button.set_text("°C");
            button.set_id("toggle_temp_unit");
            button
        });
        config_row
    });
    root.append_child({
        let mut config_row = UiElement::new(UiElementKind::Columns);
        config_row.append_child(UiElement::from_string("Power Switch Led"));
        config_row.append_child(UiElement::new(UiElementKind::Spacer));
        config_row.append_child({
            let mut button = UiElement::new(UiElementKind::Button);
            button.set_text("Disable");
            button.set_id("toggle_pwr_led");
            button
        });
        config_row
    });

    drop(root);
    ui_page.get_changes();
    ui_page
}