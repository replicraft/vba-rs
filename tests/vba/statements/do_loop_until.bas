' INV: stmt.do-loop
' SOURCE: spec ms-vbal 5.4.2.6 Do statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
    Do
        i = i + 1
    Loop Until i = 3
    Debug.Print i
End Sub
