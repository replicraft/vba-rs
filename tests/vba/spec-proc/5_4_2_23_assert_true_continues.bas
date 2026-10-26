' INV: vbal.5.4.2.23
' SOURCE: spec ms-vbal 5.4.2.23 Debug.Assert with a True expression does not suspend execution
' EXPECT: ok
Public Sub Main()
    Debug.Assert 1 = 1
    Debug.Print "ok"
End Sub
