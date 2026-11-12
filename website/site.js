const input = document.querySelector("#search");
const status = document.querySelector(".searchstatus");
const links = [...document.querySelectorAll(".docnav a")];

if (input && status) {
  input.addEventListener("input", () => {
    const query = input.value.trim().toLowerCase();
    let visible = 0;
    for (const link of links) {
      const match = !query || link.textContent.toLowerCase().includes(query);
      link.hidden = !match;
      if (match) visible += 1;
    }
    status.textContent = query ? `${visible} guide${visible === 1 ? "" : "s"}` : "";
  });
}

const terminalForm = document.querySelector("[data-terminal-form]");
const terminalInput = document.querySelector("[data-terminal-input]");
const terminalHelp = document.querySelector(".terminal-help");

if (terminalForm && terminalInput && terminalHelp) {
  const commands = {
    help: "COMMANDS: CATALOG, INSTALL, DOCS, SECURITY, CLEAR.",
    catalog: "6 GUEST SYSTEMS READY. OPENING MACHINE CATALOG…",
    install: "COPY: brew install --cask wess/packages/vista",
    docs: "OPENING VISTA OPERATOR MANUAL…",
    security: "NATIVE ISOLATION. ZERO TELEMETRY. LOCAL DATA.",
  };
  terminalForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const command = terminalInput.value.trim().toLowerCase();
    terminalInput.value = "";
    if (!command) return;
    if (command === "clear") {
      terminalHelp.innerHTML = "TYPE A COMMAND: CATALOG, INSTALL, DOCS,<br>SECURITY, OR CLEAR.";
      return;
    }
    terminalHelp.textContent = commands[command] || `UNKNOWN COMMAND: ${command.toUpperCase()}. TYPE HELP.`;
    if (command === "catalog") setTimeout(() => location.assign("/vista/docs/systems/"), 700);
    if (command === "docs") setTimeout(() => location.assign("/vista/docs/install/"), 700);
    if (command === "security") setTimeout(() => location.assign("/vista/docs/security/"), 700);
  });
}
