function Greeting(props: { name: string; show: boolean }) {
  return (
    <div>
      {props.show ? <span>Hello, {props.name}</span> : <span>Hidden</span>}
      {props.show && <em>visible</em>}
    </div>
  );
}
