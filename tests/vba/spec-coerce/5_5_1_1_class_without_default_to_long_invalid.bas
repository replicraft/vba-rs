' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, a class with no accessible default member to any type is invalid
' ERROR: load
'#MODULE Plain class
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim p As Plain
    Dim n As Long
    Set p = New Plain
    n = p
End Sub
