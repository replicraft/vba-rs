' INV: expr.typeof
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/is-operator
' EXPECT: TrueFalse
' EXPECT: Dog
'#MODULE Dog class
Public Sub Speak()
End Sub
'#MODULE Cat class
Public Sub Speak()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Set o = New Dog
    Debug.Print TypeOf o Is Dog; TypeOf o Is Cat
    Debug.Print TypeName(o)
End Sub
