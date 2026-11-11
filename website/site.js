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
