use core::{fmt::Display, sync::atomic::AtomicI32};

pub static ERRNO: AtomicI32 = AtomicI32::new(0);

#[repr(i32)]
pub enum ErrNo {
    /// エラーなし
    None = 0,

    /// 引き数リストが長過ぎる (POSIX.1)
    E2BIG,

    /// 許可がない (POSIX.1)
    EACCES,

    /// アドレスがすでに使用されている (POSIX.1)
    EADDRINUSE,

    /// アドレスが使用できない (POSIX.1)
    EADDRNOTAVAIL,

    /// アドレスファミリーがサポートされていない (POSIX.1)
    EAFNOSUPPORT,

    /// リソースが一時的に利用不可 (EWOULDBLOCK と同じ値でもよい) (POSIX.1)
    EAGAIN,

    /// 接続が既に処理中である (POSIX.1)
    EALREADY,

    /// 不正なやり取り (exchange) である
    EBADE,

    /// ファイルディスクリプターが不正である (POSIX.1)
    EBADF,

    /// ファイルディスクリプターが不正な状態である
    EBADFD,

    /// メッセージが不正である (POSIX.1)
    EBADMSG,

    /// 不正なリクエストディスクリプター
    EBADR,

    /// 不正なリクエストコード
    EBADRQC,

    /// 不正なスロット
    EBADSLT,

    /// リソースが使用中である (POSIX.1)
    EBUSY,

    /// 操作がキャンセルされた (POSIX.1)
    ECANCELED,

    /// 子プロセスが無い (POSIX.1)
    ECHILD,

    /// チャンネル番号が範囲外である
    ECHRNG,

    /// 送信時に通信エラーが発生した
    ECOMM,

    /// 接続が中止された (POSIX.1)
    ECONNABORTED,

    /// 接続が拒否された (POSIX.1)
    ECONNREFUSED,

    /// 接続がリセットされた (POSIX.1)
    ECONNRESET,

    /// リソースのデッドロックを回避した (POSIX.1)
    EDEADLK,

    /// EDEADLK の同義語
    EDEADLOCK,

    /// 宛先アドレスが必要である (POSIX.1)
    EDESTADDRREQ,

    /// 数学関数で引き数が領域外である (out of domain)
    EDOM,

    /// ディスククォータ (quota) を超過した (POSIX.1)
    EDQUOT,

    /// ファイルが存在する (POSIX.1)
    EEXIST,

    /// アドレスが不正である (POSIX.1)
    EFAULT,

    /// ファイルが大き過ぎる (POSIX.1)
    EFBIG,

    /// ホストがダウンしている
    EHOSTDOWN,

    /// ホストに到達不能である (POSIX.1)
    EHOSTUNREACH,

    /// 識別子が削除された (POSIX.1)
    EIDRM,

    /// 不正なバイト列 (POSIX.1, C99)
    EILSEQ,

    /// 操作が実行中である (POSIX.1)
    EINPROGRESS,

    /// 関数呼び出しが割り込まれた (POSIX.1); signal(7)  参照。
    EINTR,

    /// 引数が無効である (POSIX.1)
    EINVAL,

    /// 入出力エラー (POSIX.1)
    EIO,

    /// ソケットが接続されている (POSIX.1)
    EISCONN,

    /// ディレクトリである (POSIX.1)
    EISDIR,

    /// 名前付きのファイルである
    EISNAM,

    /// 鍵が期限切れとなった
    EKEYEXPIRED,

    /// 鍵がサーバにより拒否された
    EKEYREJECTED,

    /// 鍵が無効となった
    EKEYREVOKED,

    /// 停止 (レベル 2)
    EL2HLT,

    /// 同期できていない (レベル 2)
    EL2NSYNC,

    /// 停止 (レベル 3)
    EL3HLT,

    /// 停止 (レベル 3)
    EL3RST,

    /// 必要な共有ライブラリにアクセスできなかった
    ELIBACC,

    /// 壊れた共有ライブラリにアクセスしようとした
    ELIBBAD,

    /// リンクしようとした共有ライブラリが多過ぎる
    ELIBMAX,

    /// a.out のライブラリセクションが壊れている (corrupted)
    ELIBSCN,

    /// 共有ライブラリを直接実行できなかった
    ELIBEXEC,

    /// シンボリックリンクの回数が多過ぎる (POSIX.1)
    ELOOP,

    /// 間違ったメディア種別である
    EMEDIUMTYPE,

    /// オープンしているファイルが多過ぎる (POSIX.1)。
    /// 通常は getrlimit(2) に説明があるリソース上限 RLIMIT_NOFILE を超過した場合に発生する。
    EMFILE,

    /// リンクが多過ぎる (POSIX.1)
    EMLINK,

    /// メッセージが長過ぎる (POSIX.1)
    EMSGSIZE,

    /// マルチホップ (multihop) を試みた (POSIX.1)
    EMULTIHOP,

    /// ファイル名が長過ぎる (POSIX.1)
    ENAMETOOLONG,

    /// ネットワークが不通である (POSIX.1)
    ENETDOWN,

    /// 接続がネットワーク側から中止された (POSIX.1)
    ENETRESET,

    /// ネットワークが到達不能である (POSIX.1)
    ENETUNREACH,

    /// システム全体でオープンされているファイルが多過ぎる (POSIX.1)
    ENFILE,

    /// 使用可能なバッファー空間がない (POSIX.1 (XSI STREAMS option))
    ENOBUFS,

    /// ストリームの読み出しキューの先頭に読み出し可能なメッセージがない (POSIX.1)
    ENODATA,

    /// そのようなデバイスは無い (POSIX.1)
    ENODEV,

    /// そのようなファイルやディレクトリは無い (POSIX.1)
    ENOENT,

    /// 実行ファイル形式のエラー (POSIX.1)
    ENOEXEC,

    /// 要求された鍵が利用できない
    ENOKEY,

    /// 利用できるロックが無い (POSIX.1)
    ENOLCK,

    /// リンクが切れている (POSIX.1)
    ENOLINK,

    /// メディアが見つからない
    ENOMEDIUM,

    /// 十分な空きメモリー領域が無い (POSIX.1)
    ENOMEM,

    /// 要求された型のメッセージが存在しない (POSIX.1)
    ENOMSG,

    /// マシンがネットワーク上にない
    ENONET,

    /// パッケージがインストールされていない
    ENOPKG,

    /// 指定されたプロトコルが利用できない (POSIX.1)
    ENOPROTOOPT,

    /// デバイスに空き領域が無い (POSIX.1)
    ENOSPC,

    /// 指定されたストリームリソースが存在しない (POSIX.1 (XSI STREAMS option))
    ENOSR,

    /// ストリームではない (POSIX.1 (XSI STREAMS option))
    ENOSTR,

    /// 関数が実装されていない (POSIX.1)
    ENOSYS,

    /// ブロックデバイスが必要である
    ENOTBLK,

    /// ソケットが接続されていない (POSIX.1)
    ENOTCONN,

    /// ディレクトリではない (POSIX.1)
    ENOTDIR,

    /// ディレクトリが空ではない (POSIX.1)
    ENOTEMPTY,

    /// ソケットではない (POSIX.1)
    ENOTSOCK,

    /// 操作がサポートされていない (POSIX.1)
    ENOTSUP,

    /// I/O 制御操作が適切でない (POSIX.1)
    ENOTTY,

    /// 名前がネットワークで一意ではない
    ENOTUNIQ,

    /// そのようなデバイスやアドレスはない (POSIX.1)
    ENXIO,

    /// ソケットでサポートしていない操作である (POSIX.1)
    EOPNOTSUPP,

    /// 指定されたデータ型に格納するには値が大き過ぎる (POSIX.1)
    EOVERFLOW,

    /// 操作が許可されていない (POSIX.1)
    EPERM,

    /// サポートされていないプロトコルファミリーである
    EPFNOSUPPORT,

    /// パイプが壊れている (POSIX.1)
    EPIPE,

    /// プロトコルエラー (POSIX.1)
    EPROTO,

    /// プロトコルがサポートされていない (POSIX.1)
    EPROTONOSUPPORT,

    /// ソケットに指定できないプロトコルタイプである (POSIX.1)
    EPROTOTYPE,

    /// 結果が大き過ぎる (POSIX.1, C99)
    ERANGE,

    /// リモートアドレスが変わった
    EREMCHG,

    /// オブジェクトがリモートにある
    EREMOTE,

    /// リモート I/O エラー
    EREMOTEIO,

    /// システムコールが中断され再スタートが必要である
    ERESTART,

    /// 読み出し専用のファイルシステムである (POSIX.1)
    EROFS,

    /// 通信相手がシャットダウンされて送信できない
    ESHUTDOWN,

    /// 無効なシーク (POSIX.1)
    ESPIPE,

    /// サポートされていないソケット種別である
    ESOCKTNOSUPPORT,

    /// そのようなプロセスは無い (POSIX.1)
    ESRCH,

    /// ファイルハンドルが古い状態になっている (POSIX.1)
    ESTALE,

    /// ストリームパイプエラー
    ESTRPIPE,

    /// 時間が経過した (POSIX.1 (XSI STREAMS option))
    ETIME,

    /// 操作がタイムアウトした (POSIX.1)
    ETIMEDOUT,

    /// テキストファイルが使用中である (POSIX.1)
    ETXTBSY,

    /// Structure needs cleaning
    EUCLEAN,

    /// プロトコルのドライバが付与 (attach) されていない
    EUNATCH,

    /// ユーザー数が多過ぎる
    EUSERS,

    /// 操作がブロックされる見込みである (EAGAIN と同じ値でもよい) (POSIX.1)
    EWOULDBLOCK,

    /// 不適切なリンク (POSIX.1)
    EXDEV,

    /// 変換テーブルが一杯である
    EXFULL,
}

impl Display for ErrNo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::None => write!(f, "エラーは発生していない"),
            Self::E2BIG => write!(f, "引き数リストが長過ぎる (POSIX.1)"),
            Self::EACCES => write!(f, "許可がない (POSIX.1)"),
            Self::EADDRINUSE => write!(f, "アドレスがすでに使用されている (POSIX.1)"),
            Self::EADDRNOTAVAIL => write!(f, "アドレスが使用できない (POSIX.1)"),
            Self::EAFNOSUPPORT => write!(f, "アドレスファミリーがサポートされていない (POSIX.1)"),
            Self::EAGAIN => write!(f, "リソースが一時的に利用不可 (EWOULDBLOCK と同じ値でもよい) (POSIX.1)"),
            Self::EALREADY => write!(f, "接続が既に処理中である (POSIX.1)"),
            Self::EBADE => write!(f, "不正なやり取り (exchange) である"),
            Self::EBADF => write!(f, "ファイルディスクリプターが不正である (POSIX.1)"),
            Self::EBADFD => write!(f, "ファイルディスクリプターが不正な状態である"),
            Self::EBADMSG => write!(f, "メッセージが不正である (POSIX.1)"),
            Self::EBADR => write!(f, "不正なリクエストディスクリプター"),
            Self::EBADRQC => write!(f, "不正なリクエストコード"),
            Self::EBADSLT => write!(f, "不正なスロット"),
            Self::EBUSY => write!(f, "リソースが使用中である (POSIX.1)"),
            Self::ECANCELED => write!(f, "操作がキャンセルされた (POSIX.1)"),
            Self::ECHILD => write!(f, "子プロセスが無い (POSIX.1)"),
            Self::ECHRNG => write!(f, "チャンネル番号が範囲外である"),
            Self::ECOMM => write!(f, "送信時に通信エラーが発生した"),
            Self::ECONNABORTED => write!(f, "接続が中止された (POSIX.1)"),
            Self::ECONNREFUSED => write!(f, "接続が拒否された (POSIX.1)"),
            Self::ECONNRESET => write!(f, "接続がリセットされた (POSIX.1)"),
            Self::EDEADLK => write!(f, "リソースのデッドロックを回避した (POSIX.1)"),
            Self::EDEADLOCK => write!(f, "EDEADLK の同義語"),
            Self::EDESTADDRREQ => write!(f, "宛先アドレスが必要である (POSIX.1)"),
            Self::EDOM => write!(f, "数学関数で引き数が領域外である (out of domain)"),
            Self::EDQUOT => write!(f, "ディスククォータ (quota) を超過した (POSIX.1)"),
            Self::EEXIST => write!(f, "ファイルが存在する (POSIX.1)"),
            Self::EFAULT => write!(f, "アドレスが不正である (POSIX.1)"),
            Self::EFBIG => write!(f, "ファイルが大き過ぎる (POSIX.1)"),
            Self::EHOSTDOWN => write!(f, "ホストがダウンしている"),
            Self::EHOSTUNREACH => write!(f, "ホストに到達不能である (POSIX.1)"),
            Self::EIDRM => write!(f, "識別子が削除された (POSIX.1)"),
            Self::EILSEQ => write!(f, "不正なバイト列 (POSIX.1, C99)"),
            Self::EINPROGRESS => write!(f, "操作が実行中である (POSIX.1)"),
            Self::EINTR => write!(f, "関数呼び出しが割り込まれた (POSIX.1); signal(7)  参照。"),
            Self::EINVAL => write!(f, "引数が無効である (POSIX.1)"),
            Self::EIO => write!(f, "入出力エラー (POSIX.1)"),
            Self::EISCONN => write!(f, "ソケットが接続されている (POSIX.1)"),
            Self::EISDIR => write!(f, "ディレクトリである (POSIX.1)"),
            Self::EISNAM => write!(f, "名前付きのファイルである"),
            Self::EKEYEXPIRED => write!(f, "鍵が期限切れとなった"),
            Self::EKEYREJECTED => write!(f, "鍵がサーバにより拒否された"),
            Self::EKEYREVOKED => write!(f, "鍵が無効となった"),
            Self::EL2HLT => write!(f, "停止 (レベル 2)"),
            Self::EL2NSYNC => write!(f, "同期できていない (レベル 2)"),
            Self::EL3HLT => write!(f, "停止 (レベル 3)"),
            Self::EL3RST => write!(f, "停止 (レベル 3)"),
            Self::ELIBACC => write!(f, "必要な共有ライブラリにアクセスできなかった"),
            Self::ELIBBAD => write!(f, "壊れた共有ライブラリにアクセスしようとした"),
            Self::ELIBMAX => write!(f, "リンクしようとした共有ライブラリが多過ぎる"),
            Self::ELIBSCN => write!(f, "a.out のライブラリセクションが壊れている (corrupted)"),
            Self::ELIBEXEC => write!(f, "共有ライブラリを直接実行できなかった"),
            Self::ELOOP => write!(f, "シンボリックリンクの回数が多過ぎる (POSIX.1)"),
            Self::EMEDIUMTYPE => write!(f, "間違ったメディア種別である"),
            Self::EMFILE => write!(f, "オープンしているファイルが多過ぎる (POSIX.1)。 通常は getrlimit(2) に説明があるリソース上限 RLIMIT_NOFILE を超過した場合に発生する。"),
            Self::EMLINK => write!(f, "リンクが多過ぎる (POSIX.1)"),
            Self::EMSGSIZE => write!(f, "メッセージが長過ぎる (POSIX.1)"),
            Self::EMULTIHOP => write!(f, "マルチホップ (multihop) を試みた (POSIX.1)"),
            Self::ENAMETOOLONG => write!(f, "ファイル名が長過ぎる (POSIX.1)"),
            Self::ENETDOWN => write!(f, "ネットワークが不通である (POSIX.1)"),
            Self::ENETRESET => write!(f, "接続がネットワーク側から中止された (POSIX.1)"),
            Self::ENETUNREACH => write!(f, "ネットワークが到達不能である (POSIX.1)"),
            Self::ENFILE => write!(f, "システム全体でオープンされているファイルが多過ぎる (POSIX.1)"),
            Self::ENOBUFS => write!(f, "使用可能なバッファー空間がない (POSIX.1 (XSI STREAMS option))"),
            Self::ENODATA => write!(f, "ストリームの読み出しキューの先頭に読み出し可能なメッセージがない (POSIX.1)"),
            Self::ENODEV => write!(f, "そのようなデバイスは無い (POSIX.1)"),
            Self::ENOENT => write!(f, "そのようなファイルやディレクトリは無い (POSIX.1)"),
            Self::ENOEXEC => write!(f, "実行ファイル形式のエラー (POSIX.1)"),
            Self::ENOKEY => write!(f, "要求された鍵が利用できない"),
            Self::ENOLCK => write!(f, "利用できるロックが無い (POSIX.1)"),
            Self::ENOLINK => write!(f, "リンクが切れている (POSIX.1)"),
            Self::ENOMEDIUM => write!(f, "メディアが見つからない"),
            Self::ENOMEM => write!(f, "十分な空きメモリー領域が無い (POSIX.1)"),
            Self::ENOMSG => write!(f, "要求された型のメッセージが存在しない (POSIX.1)"),
            Self::ENONET => write!(f, "マシンがネットワーク上にない"),
            Self::ENOPKG => write!(f, "パッケージがインストールされていない"),
            Self::ENOPROTOOPT => write!(f, "指定されたプロトコルが利用できない (POSIX.1)"),
            Self::ENOSPC => write!(f, "デバイスに空き領域が無い (POSIX.1)"),
            Self::ENOSR => write!(f, "指定されたストリームリソースが存在しない (POSIX.1 (XSI STREAMS option))"),
            Self::ENOSTR => write!(f, "ストリームではない (POSIX.1 (XSI STREAMS option))"),
            Self::ENOSYS => write!(f, "関数が実装されていない (POSIX.1)"),
            Self::ENOTBLK => write!(f, "ブロックデバイスが必要である"),
            Self::ENOTCONN => write!(f, "ソケットが接続されていない (POSIX.1)"),
            Self::ENOTDIR => write!(f, "ディレクトリではない (POSIX.1)"),
            Self::ENOTEMPTY => write!(f, "ディレクトリが空ではない (POSIX.1)"),
            Self::ENOTSOCK => write!(f, "ソケットではない (POSIX.1)"),
            Self::ENOTSUP => write!(f, "操作がサポートされていない (POSIX.1)"),
            Self::ENOTTY => write!(f, "I/O 制御操作が適切でない (POSIX.1)"),
            Self::ENOTUNIQ => write!(f, "名前がネットワークで一意ではない"),
            Self::ENXIO => write!(f, "そのようなデバイスやアドレスはない (POSIX.1)"),
            Self::EOPNOTSUPP => write!(f, "ソケットでサポートしていない操作である (POSIX.1)"),
            Self::EOVERFLOW => write!(f, "指定されたデータ型に格納するには値が大き過ぎる (POSIX.1)"),
            Self::EPERM => write!(f, "操作が許可されていない (POSIX.1)"),
            Self::EPFNOSUPPORT => write!(f, "サポートされていないプロトコルファミリーである"),
            Self::EPIPE => write!(f, "パイプが壊れている (POSIX.1)"),
            Self::EPROTO => write!(f, "プロトコルエラー (POSIX.1)"),
            Self::EPROTONOSUPPORT => write!(f, "プロトコルがサポートされていない (POSIX.1)"),
            Self::EPROTOTYPE => write!(f, "ソケットに指定できないプロトコルタイプである (POSIX.1)"),
            Self::ERANGE => write!(f, "結果が大き過ぎる (POSIX.1, C99)"),
            Self::EREMCHG => write!(f, "リモートアドレスが変わった"),
            Self::EREMOTE => write!(f, "オブジェクトがリモートにある"),
            Self::EREMOTEIO => write!(f, "リモート I/O エラー"),
            Self::ERESTART => write!(f, "システムコールが中断され再スタートが必要である"),
            Self::EROFS => write!(f, "読み出し専用のファイルシステムである (POSIX.1)"),
            Self::ESHUTDOWN => write!(f, "通信相手がシャットダウンされて送信できない"),
            Self::ESPIPE => write!(f, "無効なシーク (POSIX.1)"),
            Self::ESOCKTNOSUPPORT => write!(f, "サポートされていないソケット種別である"),
            Self::ESRCH => write!(f, "そのようなプロセスは無い (POSIX.1)"),
            Self::ESTALE => write!(f, "ファイルハンドルが古い状態になっている (POSIX.1)"),
            Self::ESTRPIPE => write!(f, "ストリームパイプエラー"),
            Self::ETIME => write!(f, "時間が経過した (POSIX.1 (XSI STREAMS option))"),
            Self::ETIMEDOUT => write!(f, "操作がタイムアウトした (POSIX.1)"),
            Self::ETXTBSY => write!(f, "テキストファイルが使用中である (POSIX.1)"),
            Self::EUCLEAN => write!(f, "Structure needs cleaning"),
            Self::EUNATCH => write!(f, "プロトコルのドライバが付与 (attach) されていない"),
            Self::EUSERS => write!(f, "ユーザー数が多過ぎる"),
            Self::EWOULDBLOCK => write!(f, "操作がブロックされる見込みである (EAGAIN と同じ値でもよい) (POSIX.1)"),
            Self::EXDEV => write!(f, "不適切なリンク (POSIX.1)"),
            Self::EXFULL => write!(f, "変換テーブルが一杯である"),
        }
    }
}

impl From<ErrNo> for i32 {
    fn from(value: ErrNo) -> Self {
        value as _
    }
}

impl From<i32> for ErrNo {
    fn from(value: i32) -> Self {
        match value {
            v if v == Self::E2BIG as i32 => Self::E2BIG,
            v if v == Self::EACCES as i32 => Self::EACCES,
            v if v == Self::EADDRINUSE as i32 => Self::EADDRINUSE,
            v if v == Self::EADDRNOTAVAIL as i32 => Self::EADDRNOTAVAIL,
            v if v == Self::EAFNOSUPPORT as i32 => Self::EAFNOSUPPORT,
            v if v == Self::EAGAIN as i32 => Self::EAGAIN,
            v if v == Self::EALREADY as i32 => Self::EALREADY,
            v if v == Self::EBADE as i32 => Self::EBADE,
            v if v == Self::EBADF as i32 => Self::EBADF,
            v if v == Self::EBADFD as i32 => Self::EBADFD,
            v if v == Self::EBADMSG as i32 => Self::EBADMSG,
            v if v == Self::EBADR as i32 => Self::EBADR,
            v if v == Self::EBADRQC as i32 => Self::EBADRQC,
            v if v == Self::EBADSLT as i32 => Self::EBADSLT,
            v if v == Self::EBUSY as i32 => Self::EBUSY,
            v if v == Self::ECANCELED as i32 => Self::ECANCELED,
            v if v == Self::ECHILD as i32 => Self::ECHILD,
            v if v == Self::ECHRNG as i32 => Self::ECHRNG,
            v if v == Self::ECOMM as i32 => Self::ECOMM,
            v if v == Self::ECONNABORTED as i32 => Self::ECONNABORTED,
            v if v == Self::ECONNREFUSED as i32 => Self::ECONNREFUSED,
            v if v == Self::ECONNRESET as i32 => Self::ECONNRESET,
            v if v == Self::EDEADLK as i32 => Self::EDEADLK,
            v if v == Self::EDEADLOCK as i32 => Self::EDEADLOCK,
            v if v == Self::EDESTADDRREQ as i32 => Self::EDESTADDRREQ,
            v if v == Self::EDOM as i32 => Self::EDOM,
            v if v == Self::EDQUOT as i32 => Self::EDQUOT,
            v if v == Self::EEXIST as i32 => Self::EEXIST,
            v if v == Self::EFAULT as i32 => Self::EFAULT,
            v if v == Self::EFBIG as i32 => Self::EFBIG,
            v if v == Self::EHOSTDOWN as i32 => Self::EHOSTDOWN,
            v if v == Self::EHOSTUNREACH as i32 => Self::EHOSTUNREACH,
            v if v == Self::EIDRM as i32 => Self::EIDRM,
            v if v == Self::EILSEQ as i32 => Self::EILSEQ,
            v if v == Self::EINPROGRESS as i32 => Self::EINPROGRESS,
            v if v == Self::EINTR as i32 => Self::EINTR,
            v if v == Self::EINVAL as i32 => Self::EINVAL,
            v if v == Self::EIO as i32 => Self::EIO,
            v if v == Self::EISCONN as i32 => Self::EISCONN,
            v if v == Self::EISDIR as i32 => Self::EISDIR,
            v if v == Self::EISNAM as i32 => Self::EISNAM,
            v if v == Self::EKEYEXPIRED as i32 => Self::EKEYEXPIRED,
            v if v == Self::EKEYREJECTED as i32 => Self::EKEYREJECTED,
            v if v == Self::EKEYREVOKED as i32 => Self::EKEYREVOKED,
            v if v == Self::EL2HLT as i32 => Self::EL2HLT,
            v if v == Self::EL2NSYNC as i32 => Self::EL2NSYNC,
            v if v == Self::EL3HLT as i32 => Self::EL3HLT,
            v if v == Self::EL3RST as i32 => Self::EL3RST,
            v if v == Self::ELIBACC as i32 => Self::ELIBACC,
            v if v == Self::ELIBBAD as i32 => Self::ELIBBAD,
            v if v == Self::ELIBMAX as i32 => Self::ELIBMAX,
            v if v == Self::ELIBSCN as i32 => Self::ELIBSCN,
            v if v == Self::ELIBEXEC as i32 => Self::ELIBEXEC,
            v if v == Self::ELOOP as i32 => Self::ELOOP,
            v if v == Self::EMEDIUMTYPE as i32 => Self::EMEDIUMTYPE,
            v if v == Self::EMFILE as i32 => Self::EMFILE,
            v if v == Self::EMLINK as i32 => Self::EMLINK,
            v if v == Self::EMSGSIZE as i32 => Self::EMSGSIZE,
            v if v == Self::EMULTIHOP as i32 => Self::EMULTIHOP,
            v if v == Self::ENAMETOOLONG as i32 => Self::ENAMETOOLONG,
            v if v == Self::ENETDOWN as i32 => Self::ENETDOWN,
            v if v == Self::ENETRESET as i32 => Self::ENETRESET,
            v if v == Self::ENETUNREACH as i32 => Self::ENETUNREACH,
            v if v == Self::ENFILE as i32 => Self::ENFILE,
            v if v == Self::ENOBUFS as i32 => Self::ENOBUFS,
            v if v == Self::ENODATA as i32 => Self::ENODATA,
            v if v == Self::ENODEV as i32 => Self::ENODEV,
            v if v == Self::ENOENT as i32 => Self::ENOENT,
            v if v == Self::ENOEXEC as i32 => Self::ENOEXEC,
            v if v == Self::ENOKEY as i32 => Self::ENOKEY,
            v if v == Self::ENOLCK as i32 => Self::ENOLCK,
            v if v == Self::ENOLINK as i32 => Self::ENOLINK,
            v if v == Self::ENOMEDIUM as i32 => Self::ENOMEDIUM,
            v if v == Self::ENOMEM as i32 => Self::ENOMEM,
            v if v == Self::ENOMSG as i32 => Self::ENOMSG,
            v if v == Self::ENONET as i32 => Self::ENONET,
            v if v == Self::ENOPKG as i32 => Self::ENOPKG,
            v if v == Self::ENOPROTOOPT as i32 => Self::ENOPROTOOPT,
            v if v == Self::ENOSPC as i32 => Self::ENOSPC,
            v if v == Self::ENOSR as i32 => Self::ENOSR,
            v if v == Self::ENOSTR as i32 => Self::ENOSTR,
            v if v == Self::ENOSYS as i32 => Self::ENOSYS,
            v if v == Self::ENOTBLK as i32 => Self::ENOTBLK,
            v if v == Self::ENOTCONN as i32 => Self::ENOTCONN,
            v if v == Self::ENOTDIR as i32 => Self::ENOTDIR,
            v if v == Self::ENOTEMPTY as i32 => Self::ENOTEMPTY,
            v if v == Self::ENOTSOCK as i32 => Self::ENOTSOCK,
            v if v == Self::ENOTSUP as i32 => Self::ENOTSUP,
            v if v == Self::ENOTTY as i32 => Self::ENOTTY,
            v if v == Self::ENOTUNIQ as i32 => Self::ENOTUNIQ,
            v if v == Self::ENXIO as i32 => Self::ENXIO,
            v if v == Self::EOPNOTSUPP as i32 => Self::EOPNOTSUPP,
            v if v == Self::EOVERFLOW as i32 => Self::EOVERFLOW,
            v if v == Self::EPERM as i32 => Self::EPERM,
            v if v == Self::EPFNOSUPPORT as i32 => Self::EPFNOSUPPORT,
            v if v == Self::EPIPE as i32 => Self::EPIPE,
            v if v == Self::EPROTO as i32 => Self::EPROTO,
            v if v == Self::EPROTONOSUPPORT as i32 => Self::EPROTONOSUPPORT,
            v if v == Self::EPROTOTYPE as i32 => Self::EPROTOTYPE,
            v if v == Self::ERANGE as i32 => Self::ERANGE,
            v if v == Self::EREMCHG as i32 => Self::EREMCHG,
            v if v == Self::EREMOTE as i32 => Self::EREMOTE,
            v if v == Self::EREMOTEIO as i32 => Self::EREMOTEIO,
            v if v == Self::ERESTART as i32 => Self::ERESTART,
            v if v == Self::EROFS as i32 => Self::EROFS,
            v if v == Self::ESHUTDOWN as i32 => Self::ESHUTDOWN,
            v if v == Self::ESPIPE as i32 => Self::ESPIPE,
            v if v == Self::ESOCKTNOSUPPORT as i32 => Self::ESOCKTNOSUPPORT,
            v if v == Self::ESRCH as i32 => Self::ESRCH,
            v if v == Self::ESTALE as i32 => Self::ESTALE,
            v if v == Self::ESTRPIPE as i32 => Self::ESTRPIPE,
            v if v == Self::ETIME as i32 => Self::ETIME,
            v if v == Self::ETIMEDOUT as i32 => Self::ETIMEDOUT,
            v if v == Self::ETXTBSY as i32 => Self::ETXTBSY,
            v if v == Self::EUCLEAN as i32 => Self::EUCLEAN,
            v if v == Self::EUNATCH as i32 => Self::EUNATCH,
            v if v == Self::EUSERS as i32 => Self::EUSERS,
            v if v == Self::EWOULDBLOCK as i32 => Self::EWOULDBLOCK,
            v if v == Self::EXDEV as i32 => Self::EXDEV,
            v if v == Self::EXFULL as i32 => Self::EXFULL,
            _ => Self::None,
        }
    }
}
