' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 implicit declared type of function from Def directive
' EXPECT: Integer
DefInt F
Function Foo()
    Foo = 1
End Function
Public Sub Main()
    Debug.Print TypeName(Foo())
End Sub
