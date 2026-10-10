import { appInfo } from '@lupa/ipc';
import { useEffect, useState } from 'react';

/** Shows "name version" from the Rust core; renders nothing until it answers. */
export function AppVersion() {
  const [label, setLabel] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void appInfo().then((result) => {
      if (active && result.ok) setLabel(`${result.value.name} ${result.value.version}`);
    });
    return () => {
      active = false;
    };
  }, []);

  return label === null ? null : <p className="version">{label}</p>;
}
