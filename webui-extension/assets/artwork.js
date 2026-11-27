(function () {
  'use strict';

  // Mount-relative path so subpath reverse proxies (e.g. /hermes/) work cleanly.
  const ART_BASE = 'extensions/hermes-agent-teams/assets/ferris/';

  const ROLE_ART = [
    [/data|analys|metric|performance|datos|analista|métrica|rendimiento|数据|分析|指标|性能/, 'member-data.png'],
    [/resear|investig|explor|study|investigador|explorador|estudio|研究|调查|探索|调研/, 'member-researcher.png'],
    [/\bqa\b|test|verif|quality|calidad|pruebas|verificador|测试|质量|验证/, 'member-qa.png'],
    [/secur|audit|risk|threat|review|seguridad|auditor|revisor|安全|审计|审查|风险/, 'member-security.png'],
    [/design|\bui\b|\bux\b|front|theme|accessib|diseñ|interfaz|frontend|设计|前端|主题/, 'member-designer.png'],
    [/docs|writer|product|spec|docu|redactor|escritor|especif|撰写|文案|写作|文档|规范/, 'member-docs.png'],
    [/release|\bbuild\b|deploy|\bops\b|\bci\b|devops|infra|ship|coordin|despliegue|operador|发布|构建|部署|运维|协调/, 'member-operator.png'],
    [/engineer|dev\b|server|backend|\bapi\b|runtime|code|rust|python|ingenier|desarroll|program|工程|后端|服务|接口|开发|代码|编程/, 'member-engineer.png'],
  ];

  const ACTION_ART = {
    working: ART_BASE + 'action-working.png',
    idle: ART_BASE + 'action-sleeping.png',
    unknown: ART_BASE + 'action-thinking.png',
    reporting: ART_BASE + 'action-reporting.png',
    sending: ART_BASE + 'action-sending.png',
    celebrating: ART_BASE + 'action-celebrating.png',
  };

  function memberArtUrl(name, role) {
    const identity = `${name || ''} ${role || ''}`.toLowerCase();
    for (const [pattern, file] of ROLE_ART) {
      if (pattern.test(identity)) return ART_BASE + file;
    }
    return ART_BASE + 'member-default.png';
  }

  function actionArtUrl(activity) {
    return ACTION_ART[activity] || ACTION_ART.unknown;
  }

  window.HermesTeamsArtwork = {
    ART_BASE,
    LEAD_ART: ART_BASE + 'team-lead.png',
    ACTION_ART,
    memberArtUrl,
    actionArtUrl,
  };
})();
