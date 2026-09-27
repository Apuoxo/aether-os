            }
            HDA_DMA_NEXT=0;
        }

        let completed=((lp/period) as usize).min(HDA_DMA_PERIODS);
        while HDA_DMA_NEXT < completed {
            let slot=HDA_DMA_NEXT;
            let got=crate::media_player::pcm_buffer(
                core::slice::from_raw_parts_mut((HDA_DMA_PHYS+slot*4096) as *mut u8,4096));
            if got==0 {
                hda_w32(base,sd,hda_r32(base,sd)&!0x2);
                HDA_STREAM_RUNNING=false;
                serial::write_str("[AUDIO] HDA PLAY EOF\n");
                return;
            }
            crate::media_player::consume_pcm(got);
            HDA_DMA_TOTAL+=got;
            HDA_DMA_NEXT+=1;
        }
        if HDA_DMA_NEXT>=HDA_DMA_PERIODS && lp>=cbl {
            HDA_DMA_NEXT=0;
        }