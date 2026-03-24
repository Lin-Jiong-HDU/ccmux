#[cfg(test)]
mod tests {
    use ccmux::server::PtySize;

    #[test]
    fn test_pty_size() {
        let size = PtySize { cols: 80, rows: 24 };
        assert_eq!(size.cols, 80);
    }
}
