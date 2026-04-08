import express from "express";
import { spawn } from "child_process";
const app = express();
const port = 3000;

app.get("/", (req, res) => {
    res.sendFile(process.cwd() + "/public/index.html");
});

app.get("/go", (req, res) => {
    const { wr, wg, wb, or, og, ob } = req.query;
    let a = spawn("../track/target/release/track", ["../track/images/14.jpg", `${wr}`, `${wg}`, `${wb}`, `${or}`, `${og}`, `${ob}`]);

    let failsafe = setTimeout(() => {
        res.send({ok: false})
    }, 300)

    a.stdout.on("data", (data) => {
        if (data.includes("executed in: ")) {
            const time = String(data).split('executed in: ')[1].trim();

            clearTimeout(failsafe)
            res.send({ ok: true, time })
        }
    });
});

app.get("/latest.png", (req, res) => {
    res.sendFile(process.cwd() + "/out.png");
});

app.listen(port, () => {
    console.log(`Example app listening on port ${port}`);
});
