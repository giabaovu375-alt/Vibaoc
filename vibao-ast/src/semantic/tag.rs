use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tag {
    Text,
    H1,
    H2,
    H3,
    P,
    Nhan,

    Image,
    Video,
    Icon,

    Button,
    Input,
    Textarea,
    Link,

    Flex,
    Grid,
    Stack,
    Khoi,
    Cuon,
    CanGiua,
    Lop,
    DinhDau,
    DinhManHinh,

    KhoangCach,
    DuongKe,

    NhomInput,
    ChonMot,
    HopKiem,
    LuaChon,

    Form,
    Modal,
    Tabs,
    GapMo,
    BangChuyen,
    XuongTrang,
    VongQuay,
    ThanhTienTrinh,
    Bang,
    BieuDo,
    BanDo,
    ThanhDieuHuong,
    TrinhSoanThao,
}
