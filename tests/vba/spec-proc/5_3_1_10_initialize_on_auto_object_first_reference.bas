' INV: vbal.5.3.1.10
' SOURCE: spec ms-vbal 5.3.1.10 Class_Initialize runs when an auto-object variable that is Nothing is referenced
' EXPECT: declared
' EXPECT: init
' EXPECT: used
' EXPECT: init
' EXPECT: used again
'#MODULE Cls class
Public V As Long
Private Sub Class_Initialize()
    Debug.Print "init"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New Cls
    Debug.Print "declared"
    c.V = 1
    Debug.Print "used"
    Set c = Nothing
    c.V = 2
    Debug.Print "used again"
End Sub
