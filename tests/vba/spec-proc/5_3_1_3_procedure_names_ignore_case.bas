' INV: vbal.5.3.1.3
' SOURCE: derived 3.3.5 identifiers are compared without regard to case, so a procedure is found by any capitalisation of its name
' METHOD: by hand
' EXPECT: hello
' EXPECT: hello
Private Sub SayHello()
    Debug.Print "hello"
End Sub
Public Sub Main()
    sayhello
    SAYHELLO
End Sub
