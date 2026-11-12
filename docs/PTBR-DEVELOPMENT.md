# OverlaySubs PT-BR — desenvolvimento

Esta edição usa identidade própria (`io.github.gustavowin96-ai.overlaysubs.ptbr`) e nome de instalação **OverlaySubs PT-BR**. Não importa nem substitui automaticamente a instalação original. Não há versão estável publicada por este trabalho.

## Uso

1. No workflow **PT-BR Windows development** do fork `gustavowin96-ai/overlaysubs`, abra uma execução bem-sucedida da branch `dev/low-latency-pt-br` e baixe o artefato **OverlaySubs-PTBR-Windows-x64**. Extraia o ZIP e execute o instalador `.exe`.
2. Em Models, detecte o hardware e selecione **Whisper base**. Baixe Whisper e Silero. English não precisa de PLaMo nem do modelo português.
3. Para português, clique em **Baixar modelo português** (1.107.409.472 bytes). O download tem revisão fixa e SHA-256; após ele, a tradução não usa rede.
4. Escolha captura System, reproduza áudio no Brave, ative **Low Latency** e inicie a captura. Alterne English / Português (Brasil) / Bilíngue durante a reprodução.
5. **Original · PLaMo** preserva o tradutor e os idiomas originais. Para entrar/sair desse modo ou trocar modelos, pare a captura. A troca entre os três modos novos não exige parar.

Português traduz frases finalizadas; no bilíngue, o inglês parcial aparece antes. Trocar de English para português passa a traduzir as próximas frases finalizadas. Frases anteriores que não foram traduzidas não são traduzidas retroativamente. A primeira tradução inclui o carregamento do modelo e pode demorar mais.

## Diagnóstico pelo código

- No caminho original, `Session::handle` chama o tradutor para cada parcial e só então emite a legenda. A mesma thread faz ASR e MT. `overlay.js` descarta legendas sem tradução. Portanto, o inglês exibido também espera MT.
- O endpointer original exige 600 ms de silêncio, produz parciais a cada 1 s e permite até 15 s por segmento. Cada parcial retranscreve o áudio acumulado; reduzir o modelo sozinho não remove essa amplificação de trabalho.
- Há Silero no endpointer e outro VAD dentro de cada chamada Whisper. Low Latency mantém o primeiro e desliga o segundo.
- O benchmark da interface usa áudio sintético e, no modo original, soma ASR e tradução. Seu RTF não mede fala até desenho do overlay. Um valor de 0,92 não garante menos de dois segundos de atraso.

## Implementação

- Low Latency: parcial de 800 ms, silêncio de 320 ms, teto de 4 s, preservando pre-roll de 200 ms e a lógica existente de confirmação de frases e remoção de sobreposição.
- Os três modos novos emitem inglês sem chamar o tradutor na thread de reconhecimento. Português roda em outra thread, com dois threads de inferência, identificação de frase e sessão. ASR usa no máximo seis threads.
- A fila de eventos tem limite de 16; parciais obsoletos são descartados pelo consumidor. A fila de tradução tem limite de oito frases. Sob sobrecarga, o app avisa quando omite um segmento ou tradução, em vez de bloquear o reconhecimento. A fila de captura existente permanece separada e sem limite; uma falha persistente no VAD ainda pode acumular áudio.
- IDs ordenam as frases mesmo quando uma tradução antiga chega depois de um parcial novo. Resultados de uma sessão anterior não substituem os da nova sessão. A geração portuguesa observa cancelamento entre tokens.
- O modelo português é carregado sob demanda. O PLaMo não é carregado nos três modos novos. Erros e carregamento aparecem na interface.
- Instalador CPU x64 sem CUDA/Vulkan, com DLLs do llama.cpp e `GGML_NATIVE=OFF`. Vulkan permanece disponível no código original, mas não foi escolhido sem uma comparação na Radeon integrada.

## Escolha do tradutor

| Alternativa | Licença e tamanho | Avaliação de integração |
|---|---|---|
| [OPUS-MT tc-big-en-pt](https://huggingface.co/Helsinki-NLP/opus-mt-tc-big-en-pt) | CC BY 4.0; safetensors de 465.035.080 bytes, mais dois SentencePiece de aproximadamente 0,8 MB cada | Especializado em tradução; permite `>>pob<<` para português brasileiro. É menor que o Qwen escolhido, mas exigiria runtime Marian/ONNX/CTranslate2 e empacotamento adicionais; desempenho nesta CPU não medido. |
| [NLLB distilled 600M](https://huggingface.co/facebook/nllb-200-distilled-600M) | CC BY-NC 4.0; pesos PyTorch de 2.460.457.927 bytes e SentencePiece de 4.852.054 bytes | Restrição não comercial e modelo apresentado para pesquisa. Precisa de outro runtime/tokenizador; sem avaliação local de desempenho. |
| [Qwen3 1.7B](https://huggingface.co/Qwen/Qwen3-1.7B-GGUF), [quantização Q4_K_M](https://huggingface.co/unsloth/Qwen3-1.7B-GGUF) | Apache 2.0; arquivo de 1,107 GB | Reutiliza llama.cpp, arquivo único, instrução explícita de PT-BR, sem thinking. Escolhido para reduzir dependências. Não é um modelo especializado de tradução; qualidade e regionalismo exigem avaliação mais ampla. |

Não foi feito benchmark comparativo dos três modelos. A escolha é de integração e licença, não uma afirmação de superioridade de velocidade ou qualidade. O teste de Qwen verifica três frases simples; isso não comprova qualidade em gírias, nomes, fala interrompida ou Reels.

## Medição e testes

`cargo test -p overlaysubs-core` cobre segmentação, histórico e configurações; os testes novos cobrem mudança de modo sem chamada ao tradutor, parciais, pausas e fala contínua. `cargo test --release -p overlaysubs` cobre os adapters e a aplicação. `node --test scripts/overlay.test.cjs` verifica exibição sem tradução, bilíngue, ordenação e escape de HTML.

O teste ignorado `ptbr_models_smoke` baixa os modelos fixados, traduz três frases e reproduz JFK em ritmo natural, comparando reconhecimento com Low Latency desligado/ligado. A fonte WAV é fixada no commit `d2c6f05a46dacf663e357bcf0ed40f542a72825d` do whisper.cpp. O workflow o executa explicitamente antes de empacotar; relatórios ficam no artefato **PTBR-test-report**. Esse replay termina no reconhecimento, não testa WASAPI nem desenho do WebView.

O overlay grava `latency.jsonl` na pasta de configuração desta edição em `%APPDATA%`, limitado a aproximadamente 5 MB. Não contém áudio nem texto e não é transmitido. Campos:

- `onset_ms`: estimativa do início do segmento capturado até o desenho; inclui pre-roll e o tempo da fala acumulada.
- `tail_ms`: estimativa do fim do áudio reconhecido até o desenho.
- `render_ms`: emissão pelo backend até dois callbacks `requestAnimationFrame` no overlay visível.
- `translated`: indica se a atualização possui tradução.

O relógio do callback de captura ancora a posição das amostras; isso inclui filas e inferência, mas não mede com precisão o atraso anterior à entrega do áudio pelo dispositivo/driver nem o instante físico de exibição do monitor. Mudanças do relógio do sistema também afetam o resultado. Para validar a meta de 1–2 s, use a mesma gravação no Brave e compare as distribuições com Low Latency desligado/ligado. Registre também acurácia e omissões; segmentos menores podem cortar palavras ou perder contexto.

CPU/RAM do teste são amostrados a cada 250 ms no runner Windows. Esses valores incluem carregamento/download e testes sequenciais. Não representam uso simultâneo de Brave, ASR e tradução no Ryzen 5 5625U. Windows 10, WASAPI ao vivo, Radeon e latência física precisam de validação no equipamento de destino.
