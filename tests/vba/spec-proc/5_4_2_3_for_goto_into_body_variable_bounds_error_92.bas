' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 jumping into a For body, then reaching Next without initialisation, raises error 92
' EXPECT: in body
' ERROR: 92
Public Sub Main()
    Dim i As Long, n As Long
    n = 3
    GoTo Body
    For i = 1 To n
Body:
        Debug.Print "in body"
    Next i
End Sub
