' INV: stmt.option-private
' SOURCE: spec ms-vbal 5.2.1.4 Option Private directive
' EXPECT: ok
Option Private Module
Public Sub Main()
    Debug.Print "ok"
End Sub
