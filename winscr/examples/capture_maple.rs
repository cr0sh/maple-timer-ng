use winscr::gdi_capture::GdiCapturer;

fn main() {
    let mut cap = GdiCapturer::new("MapleStory", "MapleStoryClass", true).unwrap();
    println!("{:?}", cap.dimension());

    cap.capture().unwrap();

    let img = cap.get_image_buffer().unwrap();
    println!("{}x{}", img.width(), img.height());
    img.save("out/captured.jpg").unwrap();
}
