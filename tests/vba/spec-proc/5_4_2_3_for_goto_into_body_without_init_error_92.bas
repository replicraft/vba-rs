' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 jumping into a For body, then reaching Next without initialisation, raises error 92
' EXPECT: in body
' ERROR: 92
' MAX_STEPS: 100000
Public Sub Main()
    Dim i As Long
    GoTo Body
    For i = 1 To 3
Body:
        Debug.Print "in body"
    Next i
End Sub
