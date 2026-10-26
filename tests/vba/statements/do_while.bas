' INV: stmt.do-loop
' SOURCE: spec ms-vbal 5.4.2.6 Do statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
    i = 0
    Do While i < 3
        i = i + 1
    Loop
    Debug.Print i
End Sub
