' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 the variable after Next must be the loop's counter
' ERROR: load
Public Sub Main()
    Dim i As Long, j As Long
    For i = 1 To 3
    Next j
End Sub
