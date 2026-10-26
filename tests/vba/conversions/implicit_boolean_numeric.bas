' INV: expr.implicit-conv, type.boolean
' SOURCE: derived True is -1, False is 0
' EXPECT: -1 
' EXPECT:  0 
' EXPECT:  0 
Public Sub Main()
    Debug.Print CLng(True)
    Debug.Print CLng(False)
    Debug.Print True + 1
End Sub
