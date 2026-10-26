' INV: vbal.5.4.2.7, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.7 Exit Do must be inside a Do statement
' ERROR: load
Public Sub Main()
    Dim i As Long
    For i = 1 To 2
        Exit Do
    Next i
End Sub
