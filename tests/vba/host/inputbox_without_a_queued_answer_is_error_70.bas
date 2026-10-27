' INV: func.inputbox, vbal.6.1.2.8.1.12
' SOURCE: derived the embedding application supplies no answer: the dialog cannot be shown, so the call fails with error 70 as in a headless host
' METHOD: one answer queued, the second call has none
' INPUTBOX: only
' ERROR: 70
' EXPECT: only
Public Sub Main()
    Debug.Print InputBox("a")
    Debug.Print InputBox("b")
End Sub
