' INV: stmt.while-wend
' SOURCE: spec ms-vbal 5.4.2.2 While statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
    While i < 3
        i = i + 1
    Wend
    Debug.Print i
End Sub
