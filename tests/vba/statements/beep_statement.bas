' INV: stmt.beep
' SOURCE: spec ms-vbal 6.1.2.8.2.2 Beep
' EXPECT: ok
Public Sub Main()
    Beep
    Debug.Print "ok"
End Sub
