' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 for an array, the iteration variable must be Variant
' ERROR: load
Public Sub Main()
    Dim a(1 To 2) As Long
    Dim x As Long
    For Each x In a
    Next x
End Sub
