' INV: obj.debug
' SOURCE: spec ms-vbal 5.4.2.23 Assert statement
' EXPECT: ok
Public Sub Main()
    Debug.Assert True
    Debug.Print "ok"
End Sub
