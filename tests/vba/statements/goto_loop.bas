' INV: stmt.goto, lex.label
' SOURCE: spec ms-vbal 5.4.2.12 GoTo statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
Top:
    i = i + 1
    If i < 3 Then GoTo Top
    Debug.Print i
End Sub
