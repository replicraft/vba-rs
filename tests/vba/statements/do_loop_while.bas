' INV: stmt.do-loop
' SOURCE: spec ms-vbal 5.4.2.6 Do statement
' EXPECT:  11 
Public Sub Main()
    Dim i As Long
    i = 10
    Do
        i = i + 1
    Loop While i < 5
    Debug.Print i
End Sub
