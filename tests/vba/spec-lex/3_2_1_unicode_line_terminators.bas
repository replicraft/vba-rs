' INV: vbal.3.2.1
' SOURCE: spec ms-vbal 3.2.1 physical line grammar, U+2028 and U+2029 is a line terminator
' XFAIL: wrong pending
' EXPECT: a
' EXPECT: b
' EXPECT: c
Public Sub Main()
    Debug.Print "a"     Debug.Print "b"     Debug.Print "c"
End Sub
