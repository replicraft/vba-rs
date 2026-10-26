' INV: stmt.do-loop
' SOURCE: spec ms-vbal 5.4.2.6 Do statement
' EXPECT:  4 
Public Sub Main()
    Dim i As Long
    Do Until i >= 4
        i = i + 2
    Loop
    Debug.Print i
End Sub
