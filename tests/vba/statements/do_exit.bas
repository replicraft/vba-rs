' INV: stmt.do-loop, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.7 Exit Do statement
' EXPECT:  5 
Public Sub Main()
    Dim i As Long
    Do
        i = i + 1
        If i = 5 Then Exit Do
    Loop
    Debug.Print i
End Sub
