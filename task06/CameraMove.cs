using UnityEngine;

public class CameraSineMoveX : MonoBehaviour
{
    public float amplitude = 2.0f; 
    public float frequency = 1.0f;

    void Start()
    {
    }

    void Update()
    {
        float xOffset = amplitude * Mathf.Sin(2.0f * Mathf.PI * frequency * Time.time);
        Vector3 initialPosition = new Vector3(0.0f, 1.5f, -3.0f);

        transform.position = new Vector3(
            initialPosition.x + xOffset,
            initialPosition.y,
            initialPosition.z
        );

        transform.rotation = Quaternion.Euler(22.0f, 0.0f, 0.0f);
    }
}