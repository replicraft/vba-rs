' INV: vbal.3.2.1
' SOURCE: spec ms-vbal 3.2.1 physical line grammar, a lone carriage return is a line terminator
' EXPECT: a
' EXPECT: b
' EXPECT: c
Public Sub Main()
    Debug.Print "a"    Debug.Print "b"    Debug.Print "c"
End Sub
