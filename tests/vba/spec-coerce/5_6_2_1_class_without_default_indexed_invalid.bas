' INV: vbal.5.6.2.1, expr.default-member
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, indexing a specific class variable with no public default member is invalid
' ERROR: load
'#MODULE Plain class
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim p As Plain
    Dim x As Variant
    Set p = New Plain
    x = p(1)
End Sub
