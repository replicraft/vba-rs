' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, literal greater than the greatest Double value is invalid
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim c As Variant
    c = 1E999
End Sub
