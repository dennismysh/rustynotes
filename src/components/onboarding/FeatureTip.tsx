import { Component, Show } from "solid-js";
import { isTipSeen, dismissTip } from "../../lib/onboarding";

interface Props {
  id: string;
  message: string;
  shortcut?: string;
}

const FeatureTip: Component<Props> = (props) => {
  return (
    <Show when={!isTipSeen(props.id)}>
      <div class="feature-tip" role="status">
        <span class="feature-tip-icon" aria-hidden="true">{"\u2139"}</span>
        <span class="feature-tip-message">
          {props.message}
          <Show when={props.shortcut}>
            {" "}<kbd>{props.shortcut}</kbd>
          </Show>
        </span>
        <button
          class="feature-tip-dismiss"
          onClick={() => dismissTip(props.id)}
          aria-label={`Dismiss tip: ${props.message}`}
        >
          {"\u00D7"}
        </button>
      </div>
    </Show>
  );
};

export default FeatureTip;
