' INV: vbal.5.4.1.1
' SOURCE: spec ms-vbal 5.4.1.1 label uniqueness is per procedure
' EXPECT: a
' EXPECT: b
Sub A()
    GoTo Here
Here:
    Debug.Print "a"
End Sub
Public Sub Main()
    A
    GoTo Here
Here:
    Debug.Print "b"
End Sub
