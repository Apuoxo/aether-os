                    if analog_pin != 0 && output_conv != 0 {
                        // First bring up the codec endpoint.  The converter gets
                        // stream tag 1/channel 0 and 44.1k/16-bit PCM (0x4011);
                        // pin control 0x40 enables output.  These are the standard
                        // HDA stream/codec programming steps, leaving routing
                        // discovery data-driven rather than ALC269-hardcoded.
                        let codec_stream = ((codec as u32)<<28)|((output_conv as u32)<<20);
                        let _ = send_verb(codec_stream | (0x705u32<<8)); // D0
                        let _ = send_verb(codec_stream | (0x706u32<<8) | 0x10); // tag=1,ch=0
                        let _ = send_verb(codec_stream | (0x200u32<<8) | 0x4011); // 16-bit stereo/mono fmt low bits
                        let pin_cmd = ((codec as u32)<<28)|((analog_pin as u32)<<20);
                        let _ = send_verb(pin_cmd | (0x707u32<<8) | 0x40); // output enable
                        let _ = send_verb(pin_cmd | (0x705u32<<8)); // D0
                         let _ = send_verb(pin_cmd | (0x70Cu32<<8) | 0x02); // EAPD on
                        let _ = send_verb(pin_cmd | (0x300u32<<8) | 0xB000); // output amp, unmuted, gain 0
                        let conv_cmd = ((codec as u32)<<28)|((output_conv as u32)<<20);
                        // ALC269 laptop speaker path: unmute DAC 0x02 with a
                        // real output gain, matching the known-good Realtek
                        // initialization used by Linux (0xB026).
                        let _ = send_verb(conv_cmd | (0x300u32<<8) | 0xB026);
                        serial::write_str("[AUDIO] HDA ANALOG PATH PROGRAMMED PIN=");
                        serial::write_hex(analog_pin as usize);
                        serial::write_str(" CONV=");
                        serial::write_hex(output_conv as usize);
                        serial::write_str("\n");
                    }

                    // Select the first output stream exposed by GCAP.
                    let iss = ((gcap >> 8) & 0x0F) as usize;
                    let bss = ((gcap >> 3) & 0x1F) as usize;
                    let oss = ((gcap >> 12) & 0x0F) as usize;
                    if oss == 0 {
                        serial::write_str("[AUDIO] HDA NO_OUTPUT_STREAM\n");
                    } else {
                        HDA_STREAM_BASE = 0x80 + (HDA_ISS as usize) * 0x20;
                        HDA_STREAM_READY = true;
                        HDA_STREAM_RUNNING = false;
                        HDA_STREAM_FMT = 0x4011;
                        serial::write_str("[AUDIO] HDA OUTPUT_STREAM BASE=");