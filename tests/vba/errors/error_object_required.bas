' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.6.3 member resolution on Nothing is error 91
' EXPECT:  91 
Public Sub Main()
    Dim o As Object
    On Error Resume Next
    o.Foo
    Debug.Print Err.Number
End Sub
