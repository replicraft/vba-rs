' INV: vbal.5.6.8, expr.new
' SOURCE: spec ms-vbal 5.6.8 New expressions, each evaluation instantiates a new object whose declared type is the class
' EXPECT: distinct=True
' EXPECT: independent=1,2
' EXPECT: typename=Wrap
'#MODULE Wrap class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim a As Wrap, b As Wrap
    Set a = New Wrap
    Set b = New Wrap
    Debug.Print "distinct=" & CStr(Not (a Is b))
    a.V = 1: b.V = 2
    Debug.Print "independent=" & CStr(a.V) & "," & CStr(b.V)
    Debug.Print "typename=" & TypeName(New Wrap)
End Sub
