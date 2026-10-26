' INV: stmt.while-wend, stmt.goto
' SOURCE: spec ms-vbal 5.4.2.2 While statement, 5.4.2.12 GoTo statement
' EXPECT:  4 
Public Sub Main()
    Dim i As Long
    While True
        i = i + 1
        If i > 3 Then GoTo Out
    Wend
Out:
    Debug.Print i
End Sub
