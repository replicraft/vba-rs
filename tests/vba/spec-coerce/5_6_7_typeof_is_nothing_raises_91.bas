' INV: vbal.5.6.7, expr.typeof
' SOURCE: spec ms-vbal 5.6.7 TypeOf...Is expressions, a Nothing value raises error 91
' XFAIL: wrong not implemented: error 91 for TypeOf applied to Nothing
' ERROR: 91
Public Sub Main()
    Dim o As Object
    Dim b As Boolean
    b = TypeOf o Is Collection
    Debug.Print "not reached"
End Sub
