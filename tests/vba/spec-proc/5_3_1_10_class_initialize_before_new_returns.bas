' INV: vbal.5.3.1.10
' SOURCE: spec ms-vbal 5.3.1.10 Class_Initialize runs when New creates an instance, before the reference is returned
' EXPECT: before
' EXPECT: init
' EXPECT: after v=7
'#MODULE Cls class
Public V As Long
Private Sub Class_Initialize()
    Debug.Print "init"
    V = 7
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim c As Cls
    Debug.Print "before"
    Set c = New Cls
    Debug.Print "after v=" & CStr(c.V)
End Sub
