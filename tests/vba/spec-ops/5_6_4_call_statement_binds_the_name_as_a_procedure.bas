' INV: vbal.5.6.4
' SOURCE: derived 5.6.4 a name at the start of a call statement is bound as a procedure: a function called as a statement runs and its result is dropped
' METHOD: by hand
' EXPECT: ran
Private Function Work() As Long
    Debug.Print "ran"
    Work = 1
End Function
Public Sub Main()
    Work
End Sub
