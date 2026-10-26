' INV: vbal.6.1.2.4.1.12, func.timevalue
' SOURCE: spec ms-vbal 6.1.2.4.1.12 TimeValue raises an error when the string has invalid date information
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    v = TimeValue("13/45/1991 3:45 PM")
    Debug.Print "reached"
End Sub
