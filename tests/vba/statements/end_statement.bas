' INV: stmt.end
' SOURCE: spec ms-vbal 5.4.2.22 End statement
' EXPECT: a
Public Sub Main()
    Debug.Print "a"
    End
    Debug.Print "b"
End Sub
