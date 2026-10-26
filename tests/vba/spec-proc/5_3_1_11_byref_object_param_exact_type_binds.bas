' INV: vbal.5.3.1.11, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef Object parameter with an Object variable is a reference binding
' EXPECT: swapped
Sub Renew(ByRef o As Object)
    Set o = New Collection
    o.Add "x"
End Sub
Public Sub Main()
    Dim a As Object
    Renew a
    If a.Count = 1 Then Debug.Print "swapped"
End Sub
