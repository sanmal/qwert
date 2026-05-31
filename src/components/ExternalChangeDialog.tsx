interface Props {
  fileName: string;
  onReload: () => void;
  onKeep: () => void;
}

export function ExternalChangeDialog(props: Props) {
  return (
    <div class="dialog-overlay">
      <div class="dialog">
        <p>「{props.fileName}」が外部で変更されました。</p>
        <button onClick={props.onReload}>外部の変更を読み込む</button>
        <button onClick={props.onKeep}>自分の変更を保持する</button>
      </div>
    </div>
  );
}
