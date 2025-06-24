import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import './App.css';

function App() {
  const [ollamaStatus, setOllamaStatus] = useState('');
  const [isOllamaReady, setIsOllamaReady] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadingMessage, setLoadingMessage] = useState('');
  const [progress, setProgress] = useState(0);
  const [activeTab, setActiveTab] = useState('questions');
  
  // Question generation state
  const [csvPath, setCsvPath] = useState('');
  const [businessName, setBusinessName] = useState('');
  const [questions, setQuestions] = useState([]);
  
  // SWOT analysis state
  const [swotCsvPath, setSwotCsvPath] = useState('');
  const [pdfPath, setPdfPath] = useState('');
  const [swotBusinessName, setSwotBusinessName] = useState('');
  const [swotAnalysis, setSwotAnalysis] = useState('');

  useEffect(() => {
    checkOllamaStatus();
  }, []);

  const checkOllamaStatus = async () => {
    try {
      const status = await invoke('check_ollama_status');
      setOllamaStatus(status);
      setIsOllamaReady(true);
    } catch (error) {
      setOllamaStatus(error);
      setIsOllamaReady(false);
    }
  };

  const selectCsvFile = async (setter) => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'CSV Files',
          extensions: ['csv']
        }]
      });
      if (selected) {
        // Handle both v1 and v2 formats
        const path = selected.path || selected;
        setter(path);
        console.log('Selected file:', path);
      }
    } catch (error) {
      console.error('Error selecting file:', error);
      alert('Error selecting file: ' + error);
    }
  };

  const selectPdfFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'PDF Files',
          extensions: ['pdf']
        }]
      });
      if (selected) {
        // Handle both v1 and v2 formats
        const path = selected.path || selected;
        setPdfPath(path);
        console.log('Selected PDF:', path);
      }
    } catch (error) {
      console.error('Error selecting PDF:', error);
      alert('Error selecting file: ' + error);
    }
  };

  const generateQuestions = async () => {
    if (!csvPath || !businessName) {
      alert('Please select a CSV file and enter a business name');
      return;
    }

    setLoading(true);
    setProgress(0);
    setLoadingMessage('Reading CSV file...');
    
    try {
      console.log('Starting question generation...');
      console.log('CSV Path:', csvPath);
      console.log('Business Name:', businessName);
      
      // Simulate progress steps
      setProgress(20);
      setLoadingMessage('Finding company data...');
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setProgress(40);
      setLoadingMessage('Preparing AI prompt...');
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setProgress(60);
      setLoadingMessage('Generating questions with AI...');
      
      const result = await invoke('generate_followup_questions', {
        request: {
          csv_path: csvPath,
          business_name: businessName
        }
      });
      
      setProgress(90);
      setLoadingMessage('Processing results...');
      await new Promise(resolve => setTimeout(resolve, 300));
      
      console.log('Generated questions:', result);
      setQuestions(result);
      
      setProgress(100);
      setLoadingMessage('Complete!');
      
      if (result.length === 0) {
        alert('No questions were generated. Please check if the business name exists in the CSV file.');
      }
    } catch (error) {
      console.error('Error generating questions:', error);
      alert('Error generating questions: ' + error);
    } finally {
      setTimeout(() => {
        setLoading(false);
        setProgress(0);
        setLoadingMessage('');
      }, 500);
    }
  };

  const saveQuestionsToPdf = async () => {
    if (questions.length === 0) {
      alert('No questions to save');
      return;
    }

    try {
      const savePath = await save({
        filters: [{
          name: 'PDF Files',
          extensions: ['pdf']
        }],
        defaultPath: `${businessName}_questions.pdf`
      });

      if (savePath) {
        await invoke('save_questions_to_pdf', {
          questions: questions,
          businessName: businessName,
          outputPath: savePath
        });
        alert('Questions saved successfully!');
      }
    } catch (error) {
      alert('Error saving questions: ' + error);
    }
  };

  const generateSwot = async () => {
    if (!swotCsvPath || !pdfPath || !swotBusinessName) {
      alert('Please select CSV file, PDF file, and enter a business name');
      return;
    }

    setLoading(true);
    setProgress(0);
    setLoadingMessage('Reading files...');
    
    try {
      // Simulate progress steps
      setProgress(15);
      setLoadingMessage('Processing CSV data...');
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setProgress(30);
      setLoadingMessage('Extracting PDF content...');
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setProgress(50);
      setLoadingMessage('Analyzing company data...');
      await new Promise(resolve => setTimeout(resolve, 500));
      
      setProgress(70);
      setLoadingMessage('Generating SWOT analysis with AI...');
      
      const result = await invoke('generate_swot_analysis', {
        request: {
          csv_path: swotCsvPath,
          pdf_path: pdfPath,
          business_name: swotBusinessName
        }
      });
      
      setProgress(95);
      setLoadingMessage('Finalizing analysis...');
      await new Promise(resolve => setTimeout(resolve, 300));
      
      setSwotAnalysis(result);
      setProgress(100);
      setLoadingMessage('Complete!');
    } catch (error) {
      alert('Error generating SWOT analysis: ' + error);
    } finally {
      setTimeout(() => {
        setLoading(false);
        setProgress(0);
        setLoadingMessage('');
      }, 500);
    }
  };

  const saveSwotToPdf = async () => {
    if (!swotAnalysis) {
      alert('No SWOT analysis to save');
      return;
    }

    try {
      const savePath = await save({
        filters: [{
          name: 'PDF Files',
          extensions: ['pdf']
        }],
        defaultPath: `${swotBusinessName}_swot.pdf`
      });

      if (savePath) {
        await invoke('save_swot_to_pdf', {
          swotText: swotAnalysis,
          businessName: swotBusinessName,
          outputPath: savePath
        });
        alert('SWOT analysis saved successfully!');
      }
    } catch (error) {
      alert('Error saving SWOT analysis: ' + error);
    }
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>Company Analysis Tool</h1>
        <div className={`status ${isOllamaReady ? 'ready' : 'error'}`}>
          <strong>Ollama Status:</strong> {ollamaStatus}
        </div>
        {!isOllamaReady && (
          <button onClick={checkOllamaStatus} className="retry-btn">
            Retry Connection
          </button>
        )}
      </header>

      {isOllamaReady && (
        <main className="main-content">
          <div className="tabs">
            <button 
              className={activeTab === 'questions' ? 'active' : ''}
              onClick={() => setActiveTab('questions')}
            >
              Generate Questions
            </button>
            <button 
              className={activeTab === 'swot' ? 'active' : ''}
              onClick={() => setActiveTab('swot')}
            >
              SWOT Analysis
            </button>
          </div>

          {activeTab === 'questions' && (
            <div className="tab-content">
              <h2>Generate Follow-up Questions</h2>
              
              <div className="form-group">
                <label>CSV File:</label>
                <div className="file-input">
                  <input 
                    type="text" 
                    value={csvPath} 
                    readOnly 
                    placeholder="No file selected"
                  />
                  <button onClick={() => selectCsvFile(setCsvPath)}>
                    Browse
                  </button>
                </div>
              </div>

              <div className="form-group">
                <label>Business Name:</label>
                <input 
                  type="text" 
                  value={businessName}
                  onChange={(e) => setBusinessName(e.target.value)}
                  placeholder="Enter exact business name from CSV"
                />
              </div>

              <button 
                onClick={generateQuestions}
                disabled={loading || !csvPath || !businessName}
                className="generate-btn"
              >
                {loading ? 'Generating...' : 'Generate Questions'}
              </button>

              {questions.length > 0 && (
                <div className="results">
                  <div className="results-header">
                    <h3>Generated Questions ({questions.length})</h3>
                    <button onClick={saveQuestionsToPdf} className="save-btn">
                      Save as PDF
                    </button>
                  </div>
                  <div className="questions-list">
                    {questions.map((question, index) => (
                      <div key={index} className="question-item">
                        <span className="question-number">{index + 1}.</span>
                        <span className="question-text">{question}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {activeTab === 'swot' && (
            <div className="tab-content">
              <h2>Generate SWOT Analysis</h2>
              
              <div className="form-group">
                <label>CSV File (Company Data):</label>
                <div className="file-input">
                  <input 
                    type="text" 
                    value={swotCsvPath} 
                    readOnly 
                    placeholder="No file selected"
                  />
                  <button onClick={() => selectCsvFile(setSwotCsvPath)}>
                    Browse
                  </button>
                </div>
              </div>

              <div className="form-group">
                <label>PDF File (Q&A Responses):</label>
                <div className="file-input">
                  <input 
                    type="text" 
                    value={pdfPath} 
                    readOnly 
                    placeholder="No file selected"
                  />
                  <button onClick={selectPdfFile}>
                    Browse
                  </button>
                </div>
              </div>

              <div className="form-group">
                <label>Business Name:</label>
                <input 
                  type="text" 
                  value={swotBusinessName}
                  onChange={(e) => setSwotBusinessName(e.target.value)}
                  placeholder="Enter exact business name from CSV"
                />
              </div>

              <button 
                onClick={generateSwot}
                disabled={loading || !swotCsvPath || !pdfPath || !swotBusinessName}
                className="generate-btn"
              >
                {loading ? 'Generating...' : 'Generate SWOT Analysis'}
              </button>

              {swotAnalysis && (
                <div className="results">
                  <div className="results-header">
                    <h3>SWOT Analysis</h3>
                    <button onClick={saveSwotToPdf} className="save-btn">
                      Save as PDF
                    </button>
                  </div>
                  <div className="swot-content">
                    <pre>{swotAnalysis}</pre>
                  </div>
                </div>
              )}
            </div>
          )}
        </main>
      )}

      {loading && (
        <div className="loading-overlay">
          <div className="progress-container">
            <h3>{loadingMessage}</h3>
            <div className="progress-bar">
              <div 
                className="progress-fill" 
                style={{ width: `${progress}%` }}
              ></div>
            </div>
            <div className="progress-text">{progress}%</div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;