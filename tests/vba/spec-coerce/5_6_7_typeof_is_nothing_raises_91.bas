' INV: vbal.5.6.7, expr.typeof
' SOURCE: spec ms-vbal 5.6.7 TypeOf...Is expressions, a Nothing value raises error 91
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: TypeOf applied to a Nothing variable is False, not error 91 (the idiom `If TypeOf x Is T` relies on it)
' ERROR: 91
Public Sub Main()
    Dim o As Object
    Dim b As Boolean
    b = TypeOf o Is Collection
    Debug.Print "not reached"
End Sub
