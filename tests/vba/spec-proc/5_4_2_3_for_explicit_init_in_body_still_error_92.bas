' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 error 92 occurs even if the body assigns the counter explicitly
' XFAIL: wrong pending
' ERROR: 92
' MAX_STEPS: 100000
Public Sub Main()
    Dim i As Long
    GoTo Body
    For i = 1 To 3
Body:
        i = 1
    Next i
End Sub
