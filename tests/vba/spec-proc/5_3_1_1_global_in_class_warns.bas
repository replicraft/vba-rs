' INV: vbal.5.3.1.1
' SOURCE: spec ms-vbal 5.3.1.1 Global scope not allowed in class module (spec-strict: a warning by default, the program runs)
' EXPECT: ok
'#MODULE Cls class
Global Sub Foo()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print "ok"
End Sub
