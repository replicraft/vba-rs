' INV: vbal.2.5.1, decl.dim-as-new
' SOURCE: spec ms-vbal 2.5.1 automatic object instantiation, each access of an automatic instantiation variable holding Nothing creates a new instance
' EXPECT: before
' EXPECT: init
' EXPECT: hello
' EXPECT: cleared
' EXPECT: init
' EXPECT: hello
' EXPECT: False
'#MODULE Res class
Public Sub Hello()
    Debug.Print "hello"
End Sub
Private Sub Class_Initialize()
    Debug.Print "init"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim r As New Res
    Debug.Print "before"
    r.Hello
    Set r = Nothing
    Debug.Print "cleared"
    r.Hello
    Debug.Print CStr(r Is Nothing)
End Sub
