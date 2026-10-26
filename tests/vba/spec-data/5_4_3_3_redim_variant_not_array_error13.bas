' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 Variant whose value is not an array gives error 13
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    v = 5
    ReDim v(3)
End Sub
