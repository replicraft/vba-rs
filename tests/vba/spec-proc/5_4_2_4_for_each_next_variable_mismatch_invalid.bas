' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 the variable after Next must be the For Each variable
' ERROR: load
Public Sub Main()
    Dim a(1 To 2) As Long
    Dim x As Variant, y As Variant
    For Each x In a
    Next y
End Sub
