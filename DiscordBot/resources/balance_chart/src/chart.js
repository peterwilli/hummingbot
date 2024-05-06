import {
  Chart,
  TimeScale,
  LineController,
  LinearScale,
  Legend,
  PointElement,
  LineElement,
} from "chart.js";
Chart.register(
  TimeScale,
  LineController,
  LinearScale,
  Legend,
  PointElement,
  LineElement,
);
import "chartjs-adapter-moment";

const colors = {
  purple: {
    default: "rgba(149, 76, 233, 1)",
    half: "rgba(149, 76, 233, 0.5)",
    quarter: "rgba(149, 76, 233, 0.25)",
    zero: "rgba(149, 76, 233, 0)",
  },
  indigo: {
    default: "rgba(80, 102, 120, 1)",
    quarter: "rgba(80, 102, 120, 0.25)",
  },
};
const botDataStr = decodeURIComponent(window.location.hash.substring(1));
const botData = JSON.parse(botDataStr);
const ctx = document.querySelector(".chart canvas").getContext("2d");
//  {"botName":"TheBot","chartData":[{"timestamp":1713260476000,"profit":"4.25412"},{"timestamp":1713269445000,"profit":"3.67900"},{"timestamp":1713278490000,"profit":"3.68933"},{"timestamp":1713280834000,"profit":"3.67661"},{"timestamp":1713282851000,"profit":"3.62997"},{"timestamp":1713284066000,"profit":"3.68334"}]}
const gradient = ctx.createLinearGradient(0, 25, 0, 300);
gradient.addColorStop(0, colors.purple.half);
gradient.addColorStop(0.35, colors.purple.quarter);
gradient.addColorStop(1, colors.purple.zero);

let draw = LineController.prototype.draw;
LineController.prototype.draw = function () {
  let chart = this.chart;
  let ctx = chart.ctx;
  let _stroke = ctx.stroke;
  if ("shadowColor" in chart.options) {
    ctx.stroke = function () {
      ctx.save();
      ctx.shadowColor = chart.options.shadowColor;
      ctx.shadowBlur = 20;
      ctx.shadowOffsetX = 0;
      ctx.shadowOffsetY = 0;
      _stroke.apply(this, arguments);
      ctx.restore();
    };
  }
  draw.apply(this, arguments);
  ctx.stroke = _stroke;
};
Chart.defaults.color = "#ddd";

const options = {
  type: "line",
  data: {
    datasets: [
      {
        fill: true,
        label: botData.baseAsset,
        backgroundColor: gradient,
        pointRadius: 0,
        borderColor: colors.purple.default,
        data: botData.chartData.map((entry) => {
          return {
            x: entry.timestamp,
            y: entry.profit,
          };
        }),
        lineTension: 0.2,
        borderWidth: 2,
      },
    ],
  },
  options: {
    shadowColor: "#e15bff",
    responsive: true,
    maintainAspectRatio: false,
    layout: {
      padding: 10,
    },
    scales: {
      x: {
        type: "time",
        title: {
          display: true,
          text: "Time",
        },
      },
      y: {
        title: {
          display: true,
          text: "Profit (Price)",
        },
      },
    },
  },
};

new Chart(ctx, options);
