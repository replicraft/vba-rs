' INV: vbal.5.4.3.4, stmt.erase
' SOURCE: spec ms-vbal 5.4.3.4 Variant with non-array value gives error 13
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    v = 5
    Erase v
End Sub
