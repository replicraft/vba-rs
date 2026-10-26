' INV: vbal.5.2.4.1.1
' SOURCE: spec ms-vbal 5.2.4.1.1 default (Private) instancing: creatable by modules in the defining project
' EXPECT: 3
Public Sub Main()
    Dim x As New Widget
    Dim y As Widget
    Set y = New Widget
    x.v = 1
    y.v = 2
    Debug.Print CStr(x.v + y.v)
End Sub
'#MODULE Widget class
Public v As Long
